use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub struct TwitterClient {
    client: reqwest::Client,
}

impl TwitterClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct TwitterPostRequest {
    text: String,
}

#[derive(Deserialize)]
struct TwitterPostResponse {
    data: TwitterPostData,
}

#[derive(Deserialize)]
struct TwitterPostData {
    id: String,
}

#[async_trait::async_trait]
impl Platform for TwitterClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        let url = "https://api.twitter.com/2/tweets";

        let body = TwitterPostRequest {
            text: request.content,
        };

        let response = self
            .client
            .post(url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitter API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Twitter API error: {}", error_text)));
        }

        let twitter_response: TwitterPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Twitter response: {}", e)))?;

        Ok(PostResponse {
            platform_post_id: twitter_response.data.id.clone(),
            url: Some(format!(
                "https://twitter.com/i/web/status/{}",
                twitter_response.data.id
            )),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let url = "https://api.twitter.com/2/users/me";

        let response = self
            .client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitter API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "twitter"
    }
}
