use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

pub struct YouTubeClient {
    client: reqwest::Client,
}

impl YouTubeClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for YouTubeClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct YouTubeChannelListResponse {
    items: Vec<YouTubeChannelItem>,
}

#[derive(Deserialize)]
struct YouTubeChannelItem {
    id: String,
}

#[async_trait::async_trait]
impl Platform for YouTubeClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // YouTube doesn't have a direct "post text" API like other platforms.
        // The closest is creating a community post, which requires the YouTube Data API v3.
        // For now, we create a channel bulletin / community post via the activities endpoint.

        // First, get the authenticated user's channel ID
        let channel_response = self
            .client
            .get("https://www.googleapis.com/youtube/v3/channels")
            .bearer_auth(access_token)
            .query(&[("part", "id"), ("mine", "true")])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("YouTube API error: {}", e)))?;

        if !channel_response.status().is_success() {
            let error_text = channel_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "YouTube API error: {}",
                error_text
            )));
        }

        let channels: YouTubeChannelListResponse = channel_response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse YouTube response: {}", e)))?;

        let channel_id = channels
            .items
            .first()
            .map(|c| c.id.clone())
            .ok_or_else(|| Error::Platform("No YouTube channel found".to_string()))?;

        // Create a community post via the YouTube Data API
        let body = serde_json::json!({
            "snippet": {
                "channelId": channel_id,
                "type": "textPost",
                "textOriginal": request.content
            }
        });

        let response = self
            .client
            .post("https://www.googleapis.com/youtube/v3/activities")
            .bearer_auth(access_token)
            .query(&[("part", "snippet")])
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("YouTube API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "YouTube API error: {}",
                error_text
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse YouTube response: {}", e)))?;

        let post_id = result["id"].as_str().unwrap_or("unknown").to_string();

        Ok(PostResponse {
            platform_post_id: post_id.clone(),
            url: Some(format!(
                "https://www.youtube.com/channel/{}/community?lb={}",
                channel_id, post_id
            )),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let response = self
            .client
            .get("https://www.googleapis.com/youtube/v3/channels")
            .bearer_auth(access_token)
            .query(&[("part", "id"), ("mine", "true")])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("YouTube API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "youtube"
    }
}
