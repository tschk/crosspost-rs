use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{PostResponse, Strategy};
use crate::types::{PostOptions, TwitchCredentials};
use serde::Deserialize;

pub struct TwitchStrategy {
    client: reqwest::Client,
    credentials: TwitchCredentials,
}

impl TwitchStrategy {
    pub fn new(credentials: TwitchCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "Twitch access_token is required".to_string(),
            ));
        }
        if credentials.client_id.is_empty() {
            return Err(Error::Validation(
                "Twitch client_id is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            credentials,
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(TwitchCredentials {
            access_token: required_env("TWITCH_ACCESS_TOKEN")?,
            client_id: required_env("TWITCH_CLIENT_ID")?,
        })
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
impl Strategy for TwitchStrategy {
    fn name(&self) -> &str {
        "Twitch"
    }

    fn id(&self) -> &str {
        "twitch"
    }

    fn max_message_length(&self) -> usize {
        500
    }

    async fn post(&self, message: &str, _options: Option<&PostOptions>) -> Result<PostResponse> {
        // Get broadcaster's user ID
        let user_response = self
            .client
            .get("https://api.twitch.tv/helix/users")
            .bearer_auth(&self.credentials.access_token)
            .header("Client-Id", &self.credentials.client_id)
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

        let body = serde_json::json!({
            "message": message,
            "color": "primary"
        });

        let response = self
            .client
            .post("https://api.twitch.tv/helix/chat/announcements")
            .bearer_auth(&self.credentials.access_token)
            .header("Client-Id", &self.credentials.client_id)
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
            id: format!("announcement_{}", uuid::Uuid::new_v4()),
            url: Some(format!("https://twitch.tv/{}", broadcaster_id)),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get("https://api.twitch.tv/helix/users")
            .bearer_auth(&self.credentials.access_token)
            .header("Client-Id", &self.credentials.client_id)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitch API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(TwitchStrategy::new(TwitchCredentials {
            access_token: String::new(),
            client_id: "id".to_string(),
        })
        .is_err());

        assert!(TwitchStrategy::new(TwitchCredentials {
            access_token: "token".to_string(),
            client_id: String::new(),
        })
        .is_err());

        assert!(TwitchStrategy::new(TwitchCredentials {
            access_token: "token".to_string(),
            client_id: "id".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = TwitchStrategy::new(TwitchCredentials {
            access_token: "t".to_string(),
            client_id: "c".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "twitch");
        assert_eq!(s.name(), "Twitch");
        assert_eq!(s.max_message_length(), 500);
    }
}
