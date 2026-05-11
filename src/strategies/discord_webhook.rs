use crate::env::required_env;
use crate::error::{platform_response_error, Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{DiscordWebhookCredentials, PostOptions};
use serde::Deserialize;

/// Strategy for posting to Discord via webhook URL.
///
/// Supports multipart image uploads. No bot token required.
pub struct DiscordWebhookStrategy {
    client: reqwest::Client,
    credentials: DiscordWebhookCredentials,
    api_base: Option<String>,
}

impl DiscordWebhookStrategy {
    pub fn new(credentials: DiscordWebhookCredentials) -> Result<Self> {
        if !credentials
            .webhook_url
            .starts_with("https://discord.com/api/webhooks/")
        {
            return Err(Error::Validation(
                "Discord webhook URL must start with https://discord.com/api/webhooks/".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = Some(base);
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(DiscordWebhookCredentials {
            webhook_url: required_env("DISCORD_WEBHOOK_URL")?,
        })
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<PostResponse> {
        if !response.status().is_success() {
            return Err(platform_response_error(self.name(), response).await);
        }

        let msg: WebhookMessage = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse webhook response: {}", e)))?;

        Ok(PostResponse {
            id: msg.id,
            url: Some(format!(
                "https://discord.com/channels/@me/{}",
                msg.channel_id
            )),
        })
    }
}

#[derive(Deserialize)]
struct WebhookMessage {
    id: String,
    channel_id: String,
}

#[async_trait::async_trait]
impl Strategy for DiscordWebhookStrategy {
    fn name(&self) -> &str {
        "Discord Webhook"
    }

    fn id(&self) -> &str {
        "discord_webhook"
    }

    fn max_message_length(&self) -> usize {
        2000
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let url = if let Some(ref base) = self.api_base {
            format!("{}?wait=true", base)
        } else {
            format!("{}?wait=true", self.credentials.webhook_url)
        };

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
                .multipart(form)
                .send()
                .await
                .map_err(|e| Error::Platform(format!("Discord webhook error: {}", e)))?;

            return self.handle_response(response).await;
        }

        let body = serde_json::json!({
            "content": message,
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Discord webhook error: {}", e)))?;

        self.handle_response(response).await
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let url = self
            .api_base
            .as_deref()
            .unwrap_or(&self.credentials.webhook_url);
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Discord webhook error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> DiscordWebhookStrategy {
        DiscordWebhookStrategy::new(DiscordWebhookCredentials {
            webhook_url: "https://discord.com/api/webhooks/123/abc".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(DiscordWebhookStrategy::new(DiscordWebhookCredentials {
            webhook_url: "https://not-discord.com/webhook".to_string(),
        })
        .is_err());

        assert!(DiscordWebhookStrategy::new(DiscordWebhookCredentials {
            webhook_url: "https://discord.com/api/webhooks/123/abc".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = DiscordWebhookStrategy::new(DiscordWebhookCredentials {
            webhook_url: "https://discord.com/api/webhooks/123/abc".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "discord_webhook");
        assert_eq!(s.name(), "Discord Webhook");
        assert_eq!(s.max_message_length(), 2000);
    }

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": "msg123", "channel_id": "ch456"})),
            )
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let response = strategy.post("Hello Discord!", None).await.unwrap();
        assert_eq!(response.id, "msg123");
        assert_eq!(
            response.url,
            Some("https://discord.com/channels/@me/ch456".to_string())
        );
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.post("Hello!", None).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("HTTP 403"), "Got: {}", err);
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(strategy.validate_credentials().await.unwrap());
    }

    #[tokio::test]
    async fn test_validate_credentials_failure() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(!strategy.validate_credentials().await.unwrap());
    }
}
