use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

/// Telegram uses the Bot API instead of OAuth2.
/// The `access_token` format is "bot_token|chat_id".
pub struct TelegramClient {
    client: reqwest::Client,
}

impl TelegramClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for TelegramClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct TelegramResponse {
    ok: bool,
    result: Option<TelegramMessage>,
    description: Option<String>,
}

#[derive(Deserialize)]
struct TelegramMessage {
    message_id: i64,
    chat: TelegramChat,
}

#[derive(Deserialize)]
struct TelegramChat {
    #[allow(dead_code)]
    id: i64,
    username: Option<String>,
}

#[derive(Deserialize)]
struct TelegramGetMeResponse {
    ok: bool,
}

impl TelegramClient {
    /// Parse access_token as "bot_token|chat_id" (consistent delimiter)
    fn parse_token(access_token: &str) -> Result<(&str, &str)> {
        if let Some(idx) = access_token.rfind('|') {
            let bot_token = &access_token[..idx];
            let chat_id = &access_token[idx + 1..];
            if chat_id.parse::<i64>().is_ok() || chat_id.starts_with('@') {
                return Ok((bot_token, chat_id));
            }
        }
        Err(Error::Platform(
            "Telegram requires token as 'bot_token|chat_id'".to_string(),
        ))
    }
}

#[async_trait::async_trait]
impl Platform for TelegramClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        let (bot_token, chat_id) = Self::parse_token(access_token)?;

        // Handle image uploads via sendPhoto
        if let Some(ref images) = request.images {
            if let Some(first_image) = images.first() {
                let mime = first_image
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "image/jpeg".to_string());
                let ext = match mime.as_str() {
                    "image/png" => "png",
                    "image/jpeg" => "jpg",
                    "image/gif" => "gif",
                    _ => "jpg",
                };

                let photo_part = reqwest::multipart::Part::bytes(first_image.data.clone())
                    .mime_str(&mime)
                    .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?
                    .file_name(format!("photo.{}", ext));

                let mut form = reqwest::multipart::Form::new()
                    .text("chat_id", chat_id.to_string())
                    .part("photo", photo_part);

                if !request.content.is_empty() {
                    form = form.text("caption", request.content.clone());
                }

                let url = format!("https://api.telegram.org/bot{}/sendPhoto", bot_token);
                let response = self
                    .client
                    .post(&url)
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| Error::Platform(format!("Telegram API error: {}", e)))?;

                return self.handle_response(response).await;
            }
        }

        // Text-only message
        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);

        let body = serde_json::json!({
            "chat_id": chat_id,
            "text": request.content,
            "parse_mode": "HTML",
        });

        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Telegram API error: {}", e)))?;

        self.handle_response(response).await
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let (bot_token, _) = Self::parse_token(access_token)?;

        let url = format!("https://api.telegram.org/bot{}/getMe", bot_token);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Telegram API error: {}", e)))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let result: TelegramGetMeResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Telegram response: {}", e)))?;

        Ok(result.ok)
    }

    fn platform_name(&self) -> &'static str {
        "telegram"
    }

    fn max_message_length(&self) -> usize {
        4096
    }
}

impl TelegramClient {
    async fn handle_response(&self, response: reqwest::Response) -> Result<PostResponse> {
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Telegram API error: {}",
                error_text
            )));
        }

        let tg_response: TelegramResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Telegram response: {}", e)))?;

        if !tg_response.ok {
            return Err(Error::Platform(format!(
                "Telegram API error: {}",
                tg_response
                    .description
                    .unwrap_or_else(|| "Unknown".to_string())
            )));
        }

        let message = tg_response
            .result
            .ok_or_else(|| Error::Platform("No message in Telegram response".to_string()))?;

        let url = message
            .chat
            .username
            .map(|username| format!("https://t.me/{}/{}", username, message.message_id));

        Ok(PostResponse {
            platform_post_id: message.message_id.to_string(),
            url,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token() {
        let (bot, chat) = TelegramClient::parse_token("123456:ABC-DEF|@mychannel").unwrap();
        assert_eq!(bot, "123456:ABC-DEF");
        assert_eq!(chat, "@mychannel");

        let (bot, chat) = TelegramClient::parse_token("123456:ABC-DEF|-100123456789").unwrap();
        assert_eq!(bot, "123456:ABC-DEF");
        assert_eq!(chat, "-100123456789");

        assert!(TelegramClient::parse_token("invalid").is_err());
    }

    #[test]
    fn test_max_message_length() {
        let client = TelegramClient::new();
        assert_eq!(client.max_message_length(), 4096);
    }
}
