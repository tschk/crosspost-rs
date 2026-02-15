use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

/// Telegram uses the Bot API instead of OAuth2.
/// The `access_token` here is the bot token from @BotFather.
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

#[async_trait::async_trait]
impl Platform for TelegramClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // For Telegram, the "access_token" is the bot token.
        // The chat_id should be configured per connected account.
        // For channel posting, use @channel_username as chat_id.
        // This is a simplified implementation that expects chat_id in the content
        // format: "chat_id:message" or just posts to a default configured channel.

        // For now, we expect the platform_account_id (stored in ConnectedAccount)
        // to be the chat_id. The access_token is the bot token.
        // Since we don't have the chat_id here, we use a convention:
        // The bot token format is "bot_token:chat_id"
        let (bot_token, chat_id) = if let Some(idx) = access_token.rfind(':') {
            // Check if this looks like it has a chat_id appended
            let potential_chat_id = &access_token[idx + 1..];
            if potential_chat_id.parse::<i64>().is_ok() || potential_chat_id.starts_with('@') {
                (&access_token[..idx], potential_chat_id.to_string())
            } else {
                // Standard bot token format (number:hash), no chat_id
                return Err(Error::Platform(
                    "Telegram requires a chat_id. Store as 'bot_token|chat_id' in access_token"
                        .to_string(),
                ));
            }
        } else {
            return Err(Error::Platform("Invalid Telegram token format".to_string()));
        };

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

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        // Extract bot token (before the chat_id separator if present)
        let bot_token = if let Some(idx) = access_token.rfind('|') {
            &access_token[..idx]
        } else {
            access_token
        };

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
}
