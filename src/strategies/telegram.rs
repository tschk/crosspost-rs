use crate::env::required_env;
use crate::error::{platform_response_error, Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{PostOptions, TelegramCredentials};
use serde::Deserialize;

/// Strategy for posting to Telegram via Bot API.
///
/// Supports text messages, single photo uploads, and multi-photo media groups.
pub struct TelegramStrategy {
    client: reqwest::Client,
    credentials: TelegramCredentials,
    api_base: String,
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
        let api_base = format!("https://api.telegram.org/bot{}", credentials.bot_token);
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base,
        })
    }

    /// Override the API base URL (for testing with mock servers).
    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(TelegramCredentials {
            bot_token: required_env("TELEGRAM_BOT_TOKEN")?,
            chat_id: required_env("TELEGRAM_CHAT_ID")?,
        })
    }

    async fn handle_response(&self, response: reqwest::Response) -> Result<PostResponse> {
        if !response.status().is_success() {
            return Err(platform_response_error(self.name(), response).await);
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

    async fn send_photo(
        &self,
        message: &str,
        image: &crate::types::ImageEmbed,
    ) -> Result<PostResponse> {
        let mime = image
            .mime_type
            .clone()
            .unwrap_or_else(|| "image/jpeg".to_string());
        let ext = match mime.as_str() {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/gif" => "gif",
            _ => "jpg",
        };

        let photo_part = reqwest::multipart::Part::bytes(image.data.clone())
            .mime_str(&mime)
            .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?
            .file_name(format!("photo.{}", ext));

        let mut form = reqwest::multipart::Form::new()
            .text("chat_id", self.credentials.chat_id.clone())
            .part("photo", photo_part);

        if !message.is_empty() {
            form = form.text("caption", message.to_string());
        }

        let response = self
            .client
            .post(format!("{}/sendPhoto", self.api_base))
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Telegram API error: {}", e)))?;

        self.handle_response(response).await
    }

    async fn send_media_group(
        &self,
        caption: &str,
        images: &[crate::types::ImageEmbed],
    ) -> Result<PostResponse> {
        let mut media_json = Vec::new();
        let mut form =
            reqwest::multipart::Form::new().text("chat_id", self.credentials.chat_id.clone());

        for (i, img) in images.iter().take(10).enumerate() {
            let attach_name = format!("photo{}", i);
            let mime = img
                .mime_type
                .clone()
                .unwrap_or_else(|| "image/jpeg".to_string());
            let ext = match mime.as_str() {
                "image/png" => "png",
                "image/jpeg" => "jpg",
                "image/gif" => "gif",
                _ => "jpg",
            };

            let part = reqwest::multipart::Part::bytes(img.data.clone())
                .mime_str(&mime)
                .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?
                .file_name(format!("{}.{}", attach_name, ext));

            form = form.part(attach_name.clone(), part);

            let mut entry = serde_json::json!({
                "type": "photo",
                "media": format!("attach://{}", attach_name),
            });

            // Only the first item in the group gets the caption
            if i == 0 && !caption.is_empty() {
                entry["caption"] = serde_json::json!(caption);
            }

            media_json.push(entry);
        }

        form = form.text(
            "media",
            serde_json::to_string(&media_json)
                .map_err(|e| Error::Platform(format!("Failed to serialize media group: {}", e)))?,
        );

        let response = self
            .client
            .post(format!("{}/sendMediaGroup", self.api_base))
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Telegram API error: {}", e)))?;

        if !response.status().is_success() {
            return Err(platform_response_error(self.name(), response).await);
        }

        let tg_response: TelegramMediaGroupResponse = response
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

        let messages = tg_response
            .result
            .ok_or_else(|| Error::Platform("No messages in Telegram response".to_string()))?;

        let first = messages.first().ok_or_else(|| {
            Error::Platform("Empty message array in Telegram response".to_string())
        })?;

        let url = first
            .chat
            .username
            .as_ref()
            .map(|username| format!("https://t.me/{}/{}", username, first.message_id));

        Ok(PostResponse {
            id: first.message_id.to_string(),
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
struct TelegramMediaGroupResponse {
    ok: bool,
    result: Option<Vec<TelegramMessage>>,
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

        if images.len() > 1 {
            // Multiple images: use sendMediaGroup
            return self.send_media_group(message, images).await;
        }

        if let Some(first_image) = images.first() {
            // Single image: use sendPhoto
            return self.send_photo(message, first_image).await;
        }

        // Text only: use sendMessage
        let body = serde_json::json!({
            "chat_id": self.credentials.chat_id,
            "text": message,
            "parse_mode": "HTML",
        });

        let response = self
            .client
            .post(format!("{}/sendMessage", self.api_base))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Telegram API error: {}", e)))?;

        self.handle_response(response).await
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/getMe", self.api_base))
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
    use crate::types::ImageEmbed;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> TelegramStrategy {
        TelegramStrategy::new(TelegramCredentials {
            bot_token: "123:ABC".to_string(),
            chat_id: "-100123".to_string(),
        })
        .unwrap()
    }

    fn telegram_message_response() -> serde_json::Value {
        serde_json::json!({
            "ok": true,
            "result": {
                "message_id": 42,
                "chat": { "id": -100123, "username": "testchannel" }
            }
        })
    }

    fn telegram_media_group_response() -> serde_json::Value {
        serde_json::json!({
            "ok": true,
            "result": [
                {
                    "message_id": 42,
                    "chat": { "id": -100123, "username": "testchannel" }
                },
                {
                    "message_id": 43,
                    "chat": { "id": -100123, "username": "testchannel" }
                }
            ]
        })
    }

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
        let s = test_strategy();
        assert_eq!(s.id(), "telegram");
        assert_eq!(s.name(), "Telegram");
        assert_eq!(s.max_message_length(), 4096);
    }

    #[tokio::test]
    async fn test_post_text_message() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/sendMessage"))
            .respond_with(ResponseTemplate::new(200).set_body_json(telegram_message_response()))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.post("Hello Telegram!", None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, "42");
        assert_eq!(response.url.as_deref(), Some("https://t.me/testchannel/42"));
    }

    #[tokio::test]
    async fn test_post_single_photo() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/sendPhoto"))
            .respond_with(ResponseTemplate::new(200).set_body_json(telegram_message_response()))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let options = PostOptions {
            images: vec![ImageEmbed {
                data: vec![0xFF, 0xD8, 0xFF],
                alt: None,
                mime_type: Some("image/jpeg".to_string()),
                image_url: None,
            }],
        };

        let result = strategy.post("Photo caption", Some(&options)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "42");
    }

    #[tokio::test]
    async fn test_post_multiple_photos() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/sendMediaGroup"))
            .respond_with(ResponseTemplate::new(200).set_body_json(telegram_media_group_response()))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let options = PostOptions {
            images: vec![
                ImageEmbed {
                    data: vec![0xFF, 0xD8, 0xFF],
                    alt: None,
                    mime_type: Some("image/jpeg".to_string()),
                    image_url: None,
                },
                ImageEmbed {
                    data: vec![0x89, 0x50, 0x4E, 0x47],
                    alt: None,
                    mime_type: Some("image/png".to_string()),
                    image_url: None,
                },
            ],
        };

        let result = strategy.post("Multi photo", Some(&options)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "42");
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/getMe"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"ok": true, "result": {"id": 1}})),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(strategy.validate_credentials().await.unwrap());
    }
}
