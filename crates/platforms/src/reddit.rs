use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

pub struct RedditClient {
    client: reqwest::Client,
}

impl RedditClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("crosspost-rs/0.1.0")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

impl Default for RedditClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct RedditSubmitResponse {
    json: RedditSubmitJson,
}

#[derive(Deserialize)]
struct RedditSubmitJson {
    data: Option<RedditSubmitData>,
    errors: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RedditSubmitData {
    url: String,
    #[allow(dead_code)]
    id: String,
    name: String,
}

#[derive(Deserialize)]
struct RedditMeResponse {
    name: String,
}

#[async_trait::async_trait]
impl Platform for RedditClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // Reddit requires posting to a specific subreddit
        // The subreddit should be specified in the content or as metadata
        // For now, post to the user's profile (u/username)
        let me_response = self
            .client
            .get("https://oauth.reddit.com/api/v1/me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Reddit API error: {}", e)))?;

        if !me_response.status().is_success() {
            let error_text = me_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Reddit API error: {}", error_text)));
        }

        let me: RedditMeResponse = me_response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Reddit response: {}", e)))?;

        let subreddit = format!("u_{}", me.name);

        let params = [
            ("sr", subreddit.as_str()),
            ("kind", "self"),
            ("title", &request.content),
            ("text", &request.content),
            ("api_type", "json"),
        ];

        let response = self
            .client
            .post("https://oauth.reddit.com/api/submit")
            .bearer_auth(access_token)
            .form(&params)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Reddit API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Reddit API error: {}", error_text)));
        }

        let submit_response: RedditSubmitResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Reddit response: {}", e)))?;

        if !submit_response.json.errors.is_empty() {
            return Err(Error::Platform(format!(
                "Reddit submission errors: {:?}",
                submit_response.json.errors
            )));
        }

        let data = submit_response
            .json
            .data
            .ok_or_else(|| Error::Platform("Reddit returned no data".to_string()))?;

        Ok(PostResponse {
            platform_post_id: data.name,
            url: Some(data.url),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let response = self
            .client
            .get("https://oauth.reddit.com/api/v1/me")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Reddit API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "reddit"
    }

    fn max_message_length(&self) -> usize {
        40000
    }
}
