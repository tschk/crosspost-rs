use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub struct InstagramClient {
    client: reqwest::Client,
}

impl InstagramClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for InstagramClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct InstagramPostRequest {
    caption: String,
}

#[derive(Deserialize)]
struct InstagramPostResponse {
    id: String,
}

#[async_trait::async_trait]
impl Platform for InstagramClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // Instagram posting requires the Instagram Business Account ID
        // This is a simplified implementation
        let url = "https://graph.facebook.com/v18.0/me/media";

        let body = InstagramPostRequest {
            caption: request.content,
        };

        let response = self
            .client
            .post(url)
            .query(&[("access_token", access_token)])
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Instagram API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Instagram API error: {}",
                error_text
            )));
        }

        let ig_response: InstagramPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Instagram response: {}", e)))?;

        Ok(PostResponse {
            platform_post_id: ig_response.id.clone(),
            url: Some(format!("https://instagram.com/p/{}", ig_response.id)),
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
            .map_err(|e| Error::Platform(format!("Instagram API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "instagram"
    }
}
