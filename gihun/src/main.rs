use clap::{command, Parser};
use gihun_core::loaders::txt::load_txts_from_dir;
use rig::providers::{self, openai};
use gihun_core::attention::{Attention, AttentionConfig};
use anyhow::Result;

use gihun_core::character;
use gihun_core::init_logging;
use gihun_core::knowledge::KnowledgeBase;
use gihun_core::knowledge::Document;
use gihun_core::{agent::Agent, clients::twitter::TwitterClient};
use sqlite_vec::sqlite3_vec_init;
use tokio_rusqlite::ffi::sqlite3_auto_extension;
use tokio_rusqlite::Connection;
use gihun_core::loaders::pdf::load_pdfs_from_dir;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to character profile TOML file
    #[arg(long, default_value = "gihun/src/characters/gihun.toml")]
    character: String,

    /// Path to database
    #[arg(long, default_value = ":memory:")]
    db_path: String,

    /// Discord API token (can also be set via DISCORD_API_TOKEN env var)
    #[arg(long, env = "DISCORD_API_TOKEN", default_value = "")]
    discord_api_token: String,

    /// OpenAI API token (can also be set via OPENAI_API_KEY env var)
    #[arg(long, env = "OPENAI_API_KEY", default_value = "")]
    openai_api_key: String,

    /// GitHub repository URL
    #[arg(long, default_value = "https://github.com/cartridge-gg/docs")]
    github_repo: String,

    #[arg(long, default_value = "transcripts/")]
    transcript_path: String,

    /// Local path to clone GitHub repository
    #[arg(long, default_value = ".repo")]
    github_path: String,
    /// Twitter username
    #[arg(long, env = "TWITTER_USERNAME")]
    twitter_username: String,

    /// Twitter password
    #[arg(long, env = "TWITTER_PASSWORD")]
    twitter_password: String,

    /// Twitter email (optional, for 2FA)
    #[arg(long, env = "TWITTER_EMAIL")]
    twitter_email: Option<String>,

    /// Twitter 2FA code (optional)
    #[arg(long, env = "TWITTER_2FA_CODE")]
    twitter_2fa_code: Option<String>,

    /// Twitter cookie string (optional, alternative to username/password)
    #[arg(long, env = "TWITTER_COOKIE_STRING")]
    twitter_cookie_string: Option<String>,

    #[arg(long, env = "GALADRIEL_API_KEY")]
    galadriel_api_key: Option<String>,

    /// Telegram bot token
    #[arg(long, env = "TELEGRAM_BOT_TOKEN")]
    telegram_bot_token: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    dotenv::dotenv().ok();

    let args = Args::parse();

    // let repo = GitLoader::new(args.github_repo, &args.github_path)?;

    let character_content =
        std::fs::read_to_string(&args.character).expect("Failed to read character file");
    
    let character: character::Character = toml::from_str(&character_content)
        .map_err(|e| format!("Failed to parse character TOML: {}\nContent: {}", e, character_content))?;

    let oai = providers::openai::Client::new(&args.openai_api_key);
    let embedding_model = oai.embedding_model(openai::TEXT_EMBEDDING_3_LARGE);
    let completion_model = oai.completion_model(openai::GPT_4O);
    let should_respond_completion_model = oai.completion_model(openai::GPT_4O);

    // Initialize the `sqlite-vec`extension
    // See: https://alexgarcia.xyz/sqlite-vec/rust.html
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute(sqlite3_vec_init as *const ())));
    }

    let conn = Connection::open(args.db_path).await?;
    let mut knowledge = KnowledgeBase::new(conn.clone(), embedding_model).await?;

    let dialogue_dir = std::env::current_dir()?.join("./dialogue");

    let knowledge_chunks = load_txts_from_dir(dialogue_dir);
    
    let mut documents: Vec<Document> = Vec::new();

    for (i, chunks) in knowledge_chunks.iter().enumerate() {
        for (key, values) in chunks {
            for chunk in values{
                documents.push(Document {
                    id: format!("{}-{}", i, key), // Combine the index and the key (chunk.0)
                    source_id: "dialogue".to_string(),
                    content: chunk.to_string(),
                    created_at: chrono::Utc::now(),
                });
                print!("{}", chunk.to_string())
            }

        }
    }
        
    knowledge
        .add_documents(
           documents
        )
        .await?;

    let agent = Agent::new(character, completion_model, knowledge);

    let config = AttentionConfig {
        bot_names: vec![agent.character.name.clone()],
        ..Default::default()
    };
    let attention = Attention::new(config, should_respond_completion_model);
    // let telegram = TelegramClient::new(agent.clone(), attention.clone(), args.telegram_bot_token);
    // let discord = DiscordClient::new(agent.clone(), attention.clone());
    let twitter = TwitterClient::new(
        agent.clone(),
        attention.clone(),
        args.twitter_username,
        args.twitter_password,
        args.twitter_email,
        args.twitter_2fa_code,
        args.twitter_cookie_string,
    ).await?;
    

    twitter.start().await;

    Ok(())
}
