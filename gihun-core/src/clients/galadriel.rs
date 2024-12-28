use anyhow;
use reqwest;
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
pub struct GaladrielClient {
    api_key: String,
}

impl GaladrielClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    pub async fn generate_image(&self, image_prompt: String) -> Result<Vec<u8>, anyhow::Error> {
        let client = reqwest::Client::builder().build()?;
        let deadline = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + 300;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", format!("Bearer {}", self.api_key).parse()?);
        headers.insert("Content-Type", "application/json".parse()?);

        let body = json!({
            "model": "stabilityai/stable-diffusion-xl-base-1.0",
            "prompt": "Illustrate a beautiful and cute 20 year old anime mercenary girl with long red hair and red eyes and very large breasts wearing a sexy and revealing black outfit. Have her stand in a modern background. Emphasize the contrast between light and dark elements in the scene to create an enchanting atmosphere. Anime Portraits in the art style of Genshin Impact and Honkai Star Rail. Clear lines. High quality."
            "n": 1,
            "response_format": "url"
        });

        let request = client
            .request(
                reqwest::Method::POST,
                "https://api.galadriel.com/v1/images/generations",
            )
            .headers(headers)
            .json(&body);

        let response = request.send().await?;
        let image_url = response.text().await?.trim_matches('"').to_string();
        
        self.prepare_image_for_tweet(&image_url).await
    }

    pub async fn prepare_image_for_tweet(&self, image_url: &str) -> Result<Vec<u8>, anyhow::Error> {
        let client = reqwest::Client::new();
        let response = client.get(image_url).send().await?;

        Ok(response.bytes().await?.to_vec())
    }
}
