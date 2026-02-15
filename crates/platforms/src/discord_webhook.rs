use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

/// Discord Webhook client - posts messages via webhook URL
pub struct DiscordWebhookClient {
    client: reqwest::Client,
}

impl DiscordWebhookClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for DiscordWebhookClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct WebhookMessage {
    id: String,
    channel_id: String,
}

#[async_trait::async_trait]
impl Platform for DiscordWebhookClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // access_token is the full webhook URL
        let webhook_url = access_token;

        if !webhook_url.starts_with("https://discord.com/api/webhooks/") {
            return Err(Error::Platform("Invalid Discord webhook URL".to_string()));
        }

        // Append ?wait=true to get the message back
        let url = format!("{}?wait=true", webhook_url);

        if let Some(ref images) = request.images {
            if !images.is_empty() {
                // Use multipart form for image uploads
                let mut form = reqwest::multipart::Form::new().text("content", request.content);

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
        }

        // Text-only message
        let body = serde_json::json!({
            "content": request.content,
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

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        // Verify webhook URL is valid by GET-ing it
        let response = self
            .client
            .get(access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Discord webhook error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "discord_webhook"
    }

    fn max_message_length(&self) -> usize {
        2000
    }
}

impl DiscordWebhookClient {
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
            platform_post_id: msg.id,
            url: Some(format!(
                "https://discord.com/channels/@me/{}",
                msg.channel_id
            )),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_message_length() {
        let client = DiscordWebhookClient::new();
        assert_eq!(client.max_message_length(), 2000);
    }
}
