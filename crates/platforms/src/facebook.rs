use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub struct FacebookClient {
    client: reqwest::Client,
}

impl FacebookClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for FacebookClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct FacebookPostRequest {
    message: String,
}

#[derive(Deserialize)]
struct FacebookPostResponse {
    id: String,
}

#[async_trait::async_trait]
impl Platform for FacebookClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // Facebook posts are made to a Page's feed
        // This is a simplified implementation
        let url = "https://graph.facebook.com/v18.0/me/feed";

        let body = FacebookPostRequest {
            message: request.content,
        };

        let response = self
            .client
            .post(url)
            .query(&[("access_token", access_token)])
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Facebook API error: {}",
                error_text
            )));
        }

        let fb_response: FacebookPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Facebook response: {}", e)))?;

        Ok(PostResponse {
            platform_post_id: fb_response.id.clone(),
            url: Some(format!("https://facebook.com/{}", fb_response.id)),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let url = "https://graph.facebook.com/v18.0/me";

        let response = self
            .client
            .get(url)
            .query(&[("access_token", access_token)])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "facebook"
    }
}
