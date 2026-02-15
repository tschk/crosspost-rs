use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{DiscordWebhookCredentials, PostOptions};
use serde::Deserialize;

/// Strategy for posting to Discord via webhook URL.
///
/// Supports multipart image uploads. No bot token required.
pub struct DiscordWebhookStrategy {
    client: reqwest::Client,
    credentials: DiscordWebhookCredentials,
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
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(DiscordWebhookCredentials {
            webhook_url: required_env("DISCORD_WEBHOOK_URL")?,
        })
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<PostResponse> {
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Discord webhook error: {}",
                error_text
            )));
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
        let url = format!("{}?wait=true", self.credentials.webhook_url);

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
        let response = self
            .client
            .get(&self.credentials.webhook_url)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Discord webhook error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
