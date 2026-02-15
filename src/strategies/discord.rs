use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{DiscordCredentials, PostOptions};
use serde::Deserialize;

/// Strategy for posting to Discord channels via bot token.
///
/// Supports multipart image uploads.
pub struct DiscordStrategy {
    client: reqwest::Client,
    credentials: DiscordCredentials,
    api_base: String,
}

impl DiscordStrategy {
    pub fn new(credentials: DiscordCredentials) -> Result<Self> {
        if credentials.bot_token.is_empty() {
            return Err(Error::Validation(
                "Discord bot_token is required".to_string(),
            ));
        }
        if credentials.channel_id.is_empty() {
            return Err(Error::Validation(
                "Discord channel_id is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: "https://discord.com/api/v10".to_string(),
        })
    }

    /// Override the API base URL (for testing with mock servers).
    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(DiscordCredentials {
            bot_token: required_env("DISCORD_BOT_TOKEN")?,
            channel_id: required_env("DISCORD_CHANNEL_ID")?,
        })
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<PostResponse> {
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Discord API error: {}",
                error_text
            )));
        }

        let msg: DiscordMessage = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Discord response: {}", e)))?;

        let url = if let Some(guild_id) = &msg.guild_id {
            format!(
                "https://discord.com/channels/{}/{}/{}",
                guild_id, msg.channel_id, msg.id
            )
        } else {
            format!(
                "https://discord.com/channels/@me/{}/{}",
                msg.channel_id, msg.id
            )
        };

        Ok(PostResponse {
            id: msg.id,
            url: Some(url),
        })
    }
}

#[derive(Deserialize)]
struct DiscordMessage {
    id: String,
    channel_id: String,
    #[serde(default)]
    guild_id: Option<String>,
}

#[async_trait::async_trait]
impl Strategy for DiscordStrategy {
    fn name(&self) -> &str {
        "Discord"
    }

    fn id(&self) -> &str {
        "discord"
    }

    fn max_message_length(&self) -> usize {
        2000
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let url = format!(
            "{}/channels/{}/messages",
            self.api_base, self.credentials.channel_id
        );

        let images = get_images(options);
        if !images.is_empty() {
            let mut form = reqwest::multipart::Form::new().text("content", message.to_string());

            for (i, img) in images.iter().take(4).enumerate() {
                let mime = img
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "image/png".to_string());
                let ext = match mime.as_str() {
                    "image/png" => "png",
                    "image/jpeg" => "jpg",
                    "image/gif" => "gif",
                    _ => "png",
                };
                let part = reqwest::multipart::Part::bytes(img.data.clone())
                    .mime_str(&mime)
                    .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?
                    .file_name(format!("image{}.{}", i, ext));
                form = form.part(format!("files[{}]", i), part);
            }

            let response = self
                .client
                .post(&url)
                .header(
                    "Authorization",
                    format!("Bot {}", self.credentials.bot_token),
                )
                .multipart(form)
                .send()
                .await
                .map_err(|e| Error::Platform(format!("Discord API error: {}", e)))?;

            return self.handle_response(response).await;
        }

        let body = serde_json::json!({
            "content": message,
        });

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bot {}", self.credentials.bot_token),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Discord API error: {}", e)))?;

        self.handle_response(response).await
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/users/@me", self.api_base))
            .header(
                "Authorization",
                format!("Bot {}", self.credentials.bot_token),
            )
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Discord API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> DiscordStrategy {
        DiscordStrategy::new(DiscordCredentials {
            bot_token: "test-token".to_string(),
            channel_id: "ch123".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(DiscordStrategy::new(DiscordCredentials {
            bot_token: String::new(),
            channel_id: "123".to_string(),
        })
        .is_err());

        assert!(DiscordStrategy::new(DiscordCredentials {
            bot_token: "token".to_string(),
            channel_id: String::new(),
        })
        .is_err());

        assert!(DiscordStrategy::new(DiscordCredentials {
            bot_token: "token".to_string(),
            channel_id: "123".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = test_strategy();
        assert_eq!(s.id(), "discord");
        assert_eq!(s.name(), "Discord");
        assert_eq!(s.max_message_length(), 2000);
    }

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/channels/ch123/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "msg123",
                "channel_id": "ch456",
                "guild_id": "g789"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());

        let result = strategy.post("Hello Discord!", None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, "msg123");
        assert_eq!(
            response.url.as_deref(),
            Some("https://discord.com/channels/g789/ch456/msg123")
        );
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/channels/ch123/messages"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());

        let result = strategy.post("Hello!", None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Discord API error"));
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/@me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "1",
                "username": "testbot"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(strategy.validate_credentials().await.unwrap());
    }

    #[tokio::test]
    async fn test_validate_credentials_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/@me"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(!strategy.validate_credentials().await.unwrap());
    }
}
