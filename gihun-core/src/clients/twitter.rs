use crate::{
    agent::Agent,
    attention::{Attention, AttentionCommand, AttentionContext},
    knowledge::{ChannelType, KnowledgeBase, Message, Source},
};
use rand::Rng;
use rig::{
    completion::{CompletionModel, Prompt},
    embeddings::EmbeddingModel,
};
use agent_twitter_client::scraper::Scraper;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, error, info};


const MAX_TWEET_LENGTH: usize = 280;
const MAX_HISTORY_TWEETS: i64 = 10;


pub struct TwitterClient<M: CompletionModel, E: EmbeddingModel + 'static> {
    agent: Agent<M, E>,
    attention: Attention<M>,
    scraper: Scraper,
    username: String,
}

// rand chance between posting a new tweet
// quoting and storing tweet for context

impl From<agent_twitter_client::models::Tweet> for Message {
    fn from(tweet: agent_twitter_client::models::Tweet) -> Self {
        let created_at = tweet.time_parsed.unwrap_or_default();

        Self {
            id: tweet.id.clone().unwrap_or_default(),
            source: Source::Twitter,
            source_id: tweet.id.clone().unwrap_or_default(),
            channel_type: ChannelType::Text,
            channel_id: tweet.conversation_id.unwrap_or_default(),
            account_id: tweet.user_id.unwrap_or_default(),
            role: "user".to_string(),
            content: tweet.text.unwrap_or_default(),
            created_at,
        }
    }
}

impl<M: CompletionModel + 'static, E: EmbeddingModel + 'static> TwitterClient<M, E> {
    pub async fn new(
        agent: Agent<M, E>,
        attention: Attention<M>,
        username: String,
        password: String,
        email: Option<String>,
        two_factor_auth: Option<String>,
        cookie_string: Option<String>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut scraper = Scraper::new().await?;

        if let Some(cookie_str) = cookie_string {
            scraper.set_from_cookie_string(&cookie_str).await?;
        } else {
            scraper
                .login(
                    username.clone(),
                    password.clone(),
                    Some(email.unwrap_or_default()),
                    Some(two_factor_auth.unwrap_or_default()),
                )
                .await?;
        }

        Ok(Self {
            agent,
            attention,
            scraper,
            username
        })
    }


    pub async fn start(&self) {
        info!("Starting Twitter bot");
        loop {
            match self.random_number(0, 3) {
                // 50% chance for new tweets
                0 | 1  => {
                    debug!("Post new tweet");
                    if let Err(err) = self.post_new_tweet().await {
                        error!(?err, "Failed to post new tweet");
                    }
                }
                 // 50% chance for timeline
                2 | 3 => {
                    debug!("Process home timeline");
                    match self.scraper.get_home_timeline(5, Vec::new()).await {
                        Ok(tweets) => {
                            for tweet in tweets {
                                let tweet_content = tweet["legacy"]["full_text"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string();
                                let tweet_id = tweet["legacy"]["id_str"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_string();
                                match self.random_number(0, 3){
                                    0 | 1 => {
                                        self.handle_quote(&tweet_content, &tweet_id).await;
                                    }
                                    2 => {
                                        self.handle_retweet(&tweet_content, &tweet_id).await;

                                    }
                                    3 => {
                                        self.handle_like(&tweet_content, &tweet_id).await;
                                    }
                                    _ => unreachable!(),
                                }

                                tokio::time::sleep(tokio::time::Duration::from_secs(self.random_number(60, 180))).await;
                            }
                        }
                        Err(err) => {
                            error!(?err, "Failed to fetch home timeline");
                        }
                    }
                }
                _ => unreachable!(),
            }

            // Sleep between tasks
            tokio::time::sleep(tokio::time::Duration::from_secs(
                self.random_number(15 * 60, 60 * 60),
            )).await;
        }
    }

    async fn post_new_tweet(&self) -> Result<(), Box<dyn std::error::Error>> {
        let agent = self
            .agent
            .builder()
            .context(&format!(
                "Current time: {}",
                chrono::Local::now().format("%I:%M:%S %p, %Y-%m-%d")
            ))
            .context("Please keep your responses concise and under 280 characters. Use the provided document to draw inspiriation from lines that you, Gi-hun, would say.")
            .dynamic_context(4, self.agent.knowledge().clone().document_index())
            .build();
        let tweet_prompt = "Share brief thoughts or observation in one or two short sentences.";
        let response = match agent.prompt(&tweet_prompt).await {
            Ok(response) => response,
            Err(err) => {
                error!(?err, "Failed to generate response for tweet");
                return Ok(());
            }
        };
        debug!(response = %response, "Generated response for tweet");


        self.scraper.send_tweet(&response, None, None).await?;

        
        Ok(())
    }


    async fn handle_mention(
        &self,
        tweet: agent_twitter_client::models::Tweet,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tweet_text = Arc::new(tweet.text.clone().unwrap_or_default());
        let knowledge = self.agent.knowledge();
        let knowledge_msg = Message::from(tweet.clone());

        if let Err(err) = knowledge.create_message(knowledge_msg.clone()).await {
            error!(?err, "Failed to store tweet");
            return Ok(());
        }

        let thread = self.build_conversation_thread(&tweet).await?;

        let mentioned_names: HashSet<String> = tweet
            .text
            .unwrap_or_default()
            .split_whitespace()
            .filter(|word| word.starts_with('@'))
            .map(|mention| mention[1..].to_string())
            .collect();

        debug!(
            mentioned_names = ?mentioned_names,
            "Mentioned names in tweet"
        );

        let history = thread
            .iter()
            .map(|t| {
                (
                    t.id.clone().unwrap_or_default(),
                    t.text.clone().unwrap_or_default(),
                )
            })
            .collect();
        debug!(history = ?history, "History");
        let context = AttentionContext {
            message_content: tweet_text.as_str().to_string(),
            mentioned_names,
            history,
            channel_type: knowledge_msg.channel_type,
            source: knowledge_msg.source,
        };

        if self.username.to_lowercase() == tweet.username.unwrap_or_default().to_lowercase() {
            debug!("Not replying to bot itself");
            return Ok(());
        }

        match self.attention.should_reply(&context).await {
            AttentionCommand::Respond => {}
            _ => {
                debug!("Bot decided not to reply to tweet");
                return Ok(());
            }
        }



        let agent = self
            .agent
            .builder()
            .context(&format!(
                "Current time: {}",
                chrono::Local::now().format("%I:%M:%S %p, %Y-%m-%d")
            ))
            .context("Please keep your responses concise and under 280 characters.")
            .context("Respond naturally and conversationally in 1-2 short sentences. Avoid flowery language and excessive punctuation.")
            .context("If the tweet contains images, read it and incorporate them into your response.")
            .build();

        let response = match agent.prompt(&tweet_text.as_str().to_string()).await {
            Ok(response) => response,
            Err(err) => {
                error!(?err, "Failed to generate response");
                return Ok(());
            }
        };

        debug!(response = %response, "Generated response for reply");

        // Split response into tweet-sized chunks if necessary
        let chunks: Vec<String> = response
            .chars()
            .collect::<Vec<char>>()
            .chunks(MAX_TWEET_LENGTH)
            .map(|chunk| chunk.iter().collect::<String>())
            .collect();

        // Reply to the original tweet


        Ok(())
    }

    async fn build_conversation_thread(
        &self,
        tweet: &agent_twitter_client::models::Tweet,
    ) -> Result<Vec<agent_twitter_client::models::Tweet>, Box<dyn std::error::Error>> {
        let mut thread = Vec::new();
        let mut current_tweet = Some(tweet.clone());
        let mut depth = 0;

        debug!(
            initial_tweet_id = ?tweet.id,
            "Building conversation thread"
        );

        while let Some(tweet) = current_tweet {
            thread.push(tweet.clone());

            if depth >= MAX_HISTORY_TWEETS {
                debug!("Reached maximum thread depth of {}", MAX_HISTORY_TWEETS);
                break;
            }

            current_tweet = match tweet.in_reply_to_status_id {
                Some(parent_id) => {
                    debug!(parent_id = ?parent_id, "Fetching parent tweet");
                    match self.scraper.get_tweet(&parent_id).await {
                        Ok(parent_tweet) => Some(parent_tweet),
                        Err(err) => {
                            debug!(?err, "Failed to fetch parent tweet, stopping thread");
                            None
                        }
                    }
                }
                None => {
                    debug!("No parent tweet found, ending thread");
                    None
                }
            };

            depth += 1;
        }

        debug!(
            thread_length = thread.len(),
            depth,
            "Completed thread building"
        );
        
        thread.reverse();
        Ok(thread)
    }

    fn random_number(&self, min: u64, max: u64) -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(min..=max)
    }

    async fn handle_like(&self, tweet_content: &str, tweet_id: &str) {
        if self.attention.should_like(tweet_content).await {
            debug!(tweet_content = %tweet_content, "Agent decided to like tweet");
            if let Err(err) = self.scraper.like_tweet(tweet_id).await {
                error!(?err, "Failed to like tweet");
            }
        } else {
            debug!(tweet_content = %tweet_content, "Agent decided not to like tweet");
        }
    }

    async fn handle_retweet(&self, tweet_content: &str, tweet_id: &str) {
        if self.attention.should_retweet(tweet_content).await {
            debug!(tweet_content = %tweet_content, "Agent decided to retweet");
            if let Err(err) = self.scraper.retweet(tweet_id).await {
                error!(?err, "Failed to retweet");
            }
        } else {
            debug!(tweet_content = %tweet_content, "Agent decided not to retweet");
        }
    }
    //store quoted tweets in messages
    async fn handle_quote(&self, tweet_content: &str, tweet_id: &str) {
        
        if self.attention.should_quote(tweet_content).await {
            debug!(tweet_content = %tweet_content, "Agent decided to quote tweet");

            let agent = self
                .agent
                .builder()
                .context(&format!(
                    "Current time: {}",
                    chrono::Local::now().format("%I:%M:%S %p, %Y-%m-%d")
                ))
                .context("Please keep your responses concise and under 280 characters.")
                .context("Write a natural reply to the quoted tweet in 1-2 short sentences. Use the provided document to find similar lines that you, Gi-hun, would say. Keep it conversational and relevant.")
                .dynamic_context(4, self.agent.knowledge().clone().document_index())
                .build();

            let response = match agent.prompt(&tweet_content).await {
                Ok(response) => response,
                Err(err) => {
                    error!(?err, "Failed to generate response");
                    return;
                }
            };
            if let Err(err) = self.scraper.send_quote_tweet(&response, tweet_id, None).await {
                error!(?err, "Failed to quote tweet");
            }
        } else {
            debug!(tweet_content = %tweet_content, "Agent decided not to quote tweet");
        }
    }


}