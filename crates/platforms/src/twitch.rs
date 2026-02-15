use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

pub struct TwitchClient {
    client: reqwest::Client,
    client_id: String,
}

impl TwitchClient {
    pub fn new(client_id: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            client_id,
        }
    }
}

#[derive(Deserialize)]
struct TwitchUserResponse {
    data: Vec<TwitchUser>,
}

#[derive(Deserialize)]
struct TwitchUser {
    id: String,
}

#[async_trait::async_trait]
impl Platform for TwitchClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // Get the broadcaster's user ID
        let user_response = self
            .client
            .get("https://api.twitch.tv/helix/users")
            .bearer_auth(access_token)
            .header("Client-Id", &self.client_id)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitch API error: {}", e)))?;

        if !user_response.status().is_success() {
            let error_text = user_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Twitch API error: {}", error_text)));
        }

        let users: TwitchUserResponse = user_response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Twitch response: {}", e)))?;

        let broadcaster_id = users
            .data
            .first()
            .map(|u| u.id.clone())
            .ok_or_else(|| Error::Platform("No Twitch user found".to_string()))?;

        // Send a chat announcement (channel:manage:broadcast scope required)
        let body = serde_json::json!({
            "message": request.content,
            "color": "primary"
        });

        let response = self
            .client
            .post("https://api.twitch.tv/helix/chat/announcements")
            .bearer_auth(access_token)
            .header("Client-Id", &self.client_id)
            .query(&[
                ("broadcaster_id", broadcaster_id.as_str()),
                ("moderator_id", broadcaster_id.as_str()),
            ])
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitch API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Twitch API error: {}", error_text)));
        }

        Ok(PostResponse {
            platform_post_id: format!("announcement_{}", chrono::Utc::now().timestamp()),
            url: Some(format!("https://twitch.tv/{}", broadcaster_id)),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let response = self
            .client
            .get("https://api.twitch.tv/helix/users")
            .bearer_auth(access_token)
            .header("Client-Id", &self.client_id)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitch API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "twitch"
    }
}
