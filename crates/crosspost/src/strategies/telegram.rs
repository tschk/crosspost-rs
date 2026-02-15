use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{PostOptions, TelegramCredentials};
use serde::Deserialize;

/// Strategy for posting to Telegram via Bot API.
///
/// Supports text messages and photo uploads.
pub struct TelegramStrategy {
    client: reqwest::Client,
    credentials: TelegramCredentials,
}

impl TelegramStrategy {
    pub fn new(credentials: TelegramCredentials) -> Result<Self> {
        if credentials.bot_token.is_empty() {
            return Err(Error::Validation(
                "Telegram bot_token is required".to_string(),
            ));
        }
        if credentials.chat_id.is_empty() {
            return Err(Error::Validation(
                "Telegram chat_id is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            credentials,
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(TelegramCredentials {
            bot_token: required_env("TELEGRAM_BOT_TOKEN")?,
            chat_id: required_env("TELEGRAM_CHAT_ID")?,
        })
    }

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
            id: message.message_id.to_string(),
            url,
        })
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

#[async_trait::async_trait]
impl Strategy for TelegramStrategy {
    fn name(&self) -> &str {
        "Telegram"
    }

    fn id(&self) -> &str {
        "telegram"
    }

    fn max_message_length(&self) -> usize {
        4096
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let images = get_images(options);

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
                .text("chat_id", self.credentials.chat_id.clone())
                .part("photo", photo_part);

            if !message.is_empty() {
                form = form.text("caption", message.to_string());
            }

            let url = format!(
                "https://api.telegram.org/bot{}/sendPhoto",
                self.credentials.bot_token
            );
            let response = self
                .client
                .post(&url)
                .multipart(form)
                .send()
                .await
                .map_err(|e| Error::Platform(format!("Telegram API error: {}", e)))?;

            return self.handle_response(response).await;
        }

        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.credentials.bot_token
        );

        let body = serde_json::json!({
            "chat_id": self.credentials.chat_id,
            "text": message,
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

    async fn validate_credentials(&self) -> Result<bool> {
        let url = format!(
            "https://api.telegram.org/bot{}/getMe",
            self.credentials.bot_token
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Telegram API error: {}", e)))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        #[derive(Deserialize)]
        struct GetMeResponse {
            ok: bool,
        }

        let result: GetMeResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Telegram response: {}", e)))?;

        Ok(result.ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(TelegramStrategy::new(TelegramCredentials {
            bot_token: String::new(),
            chat_id: "123".to_string(),
        })
        .is_err());

        assert!(TelegramStrategy::new(TelegramCredentials {
            bot_token: "token".to_string(),
            chat_id: String::new(),
        })
        .is_err());

        assert!(TelegramStrategy::new(TelegramCredentials {
            bot_token: "123:ABC".to_string(),
            chat_id: "-100123".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = TelegramStrategy::new(TelegramCredentials {
            bot_token: "t".to_string(),
            chat_id: "c".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "telegram");
        assert_eq!(s.name(), "Telegram");
        assert_eq!(s.max_message_length(), 4096);
    }
}
