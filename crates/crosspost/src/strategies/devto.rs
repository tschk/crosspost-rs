use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{DevtoCredentials, PostOptions};
use base64::Engine;
use serde::Deserialize;

pub struct DevtoStrategy {
    client: reqwest::Client,
    credentials: DevtoCredentials,
}

impl DevtoStrategy {
    pub fn new(credentials: DevtoCredentials) -> Result<Self> {
        if credentials.api_key.is_empty() {
            return Err(Error::Validation("Dev.to api_key is required".to_string()));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            credentials,
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(DevtoCredentials {
            api_key: required_env("DEVTO_API_KEY")?,
        })
    }
}

#[derive(Deserialize)]
struct DevtoArticle {
    id: i64,
    url: String,
}

#[async_trait::async_trait]
impl Strategy for DevtoStrategy {
    fn name(&self) -> &str {
        "Dev.to"
    }

    fn id(&self) -> &str {
        "devto"
    }

    fn max_message_length(&self) -> usize {
        usize::MAX
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        // First line is the title, rest is body markdown
        let (title, body_markdown) = if let Some(newline_pos) = message.find('\n') {
            let title = message[..newline_pos].trim().to_string();
            let body = message[newline_pos + 1..].trim().to_string();
            (title, body)
        } else {
            (message.to_string(), String::new())
        };

        // Append images as markdown base64 tags
        let images = get_images(options);
        let mut full_body = body_markdown;
        for img in images {
            let alt = img.alt.clone().unwrap_or_else(|| "image".to_string());
            let mime = img
                .mime_type
                .clone()
                .unwrap_or_else(|| "image/png".to_string());
            let b64 = base64::engine::general_purpose::STANDARD.encode(&img.data);
            full_body.push_str(&format!("\n\n![{}](data:{};base64,{})", alt, mime, b64));
        }

        let body = serde_json::json!({
            "article": {
                "title": title,
                "body_markdown": full_body,
                "published": true,
            }
        });

        let response = self
            .client
            .post("https://dev.to/api/articles")
            .header("api-key", &self.credentials.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Dev.to API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Dev.to API error: {}", error_text)));
        }

        let article: DevtoArticle = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Dev.to response: {}", e)))?;

        Ok(PostResponse {
            id: article.id.to_string(),
            url: Some(article.url),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get("https://dev.to/api/users/me")
            .header("api-key", &self.credentials.api_key)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Dev.to API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(DevtoStrategy::new(DevtoCredentials {
            api_key: String::new(),
        })
        .is_err());

        assert!(DevtoStrategy::new(DevtoCredentials {
            api_key: "key".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = DevtoStrategy::new(DevtoCredentials {
            api_key: "k".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "devto");
        assert_eq!(s.name(), "Dev.to");
        assert_eq!(s.max_message_length(), usize::MAX);
    }
}
