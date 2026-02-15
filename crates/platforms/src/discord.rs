use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

/// Discord Bot client - posts messages to channels using bot token
pub struct DiscordClient {
    client: reqwest::Client,
}

impl DiscordClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for DiscordClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct DiscordMessage {
    id: String,
    channel_id: String,
    #[serde(default)]
    guild_id: Option<String>,
}

#[derive(Deserialize)]
struct DiscordUser {
    #[allow(dead_code)]
    id: String,
}

impl DiscordClient {
    /// Parse access_token as "bot_token|channel_id"
    fn parse_token(access_token: &str) -> Result<(&str, &str)> {
        let parts: Vec<&str> = access_token.splitn(2, '|').collect();
        if parts.len() != 2 {
            return Err(Error::Platform(
                "Discord bot requires token as 'bot_token|channel_id'".to_string(),
            ));
        }
        Ok((parts[0], parts[1]))
    }
}

#[async_trait::async_trait]
impl Platform for DiscordClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        let (bot_token, channel_id) = Self::parse_token(access_token)?;

        let url = format!(
            "https://discord.com/api/v10/channels/{}/messages",
            channel_id
        );

        // Build request body
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
                    .header("Authorization", format!("Bot {}", bot_token))
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| Error::Platform(format!("Discord API error: {}", e)))?;

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
            .header("Authorization", format!("Bot {}", bot_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Discord API error: {}", e)))?;

        self.handle_response(response).await
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let (bot_token, _) = Self::parse_token(access_token)?;

        let response = self
            .client
            .get("https://discord.com/api/v10/users/@me")
            .header("Authorization", format!("Bot {}", bot_token))
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Discord API error: {}", e)))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let _user: DiscordUser = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Discord response: {}", e)))?;

        Ok(true)
    }

    fn platform_name(&self) -> &'static str {
        "discord"
    }

    fn max_message_length(&self) -> usize {
        2000
    }
}

impl DiscordClient {
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
            platform_post_id: msg.id,
            url: Some(url),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token() {
        let (token, channel) = DiscordClient::parse_token("bottoken123|987654321").unwrap();
        assert_eq!(token, "bottoken123");
        assert_eq!(channel, "987654321");

        assert!(DiscordClient::parse_token("invalid").is_err());
    }

    #[test]
    fn test_max_message_length() {
        let client = DiscordClient::new();
        assert_eq!(client.max_message_length(), 2000);
    }
}
