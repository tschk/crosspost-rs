use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{DevtoCredentials, PostOptions};
use base64::Engine;
use serde::Deserialize;

/// Strategy for publishing articles to Dev.to.
///
/// First line of the message becomes the title, rest becomes the body.
/// Images are embedded as base64 data URIs in markdown.
pub struct DevtoStrategy {
    client: reqwest::Client,
    credentials: DevtoCredentials,
    api_base: String,
}

impl DevtoStrategy {
    pub fn new(credentials: DevtoCredentials) -> Result<Self> {
        if credentials.api_key.is_empty() {
            return Err(Error::Validation("Dev.to api_key is required".to_string()));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: "https://dev.to/api".to_string(),
        })
    }

    /// Override the API base URL (for testing with mock servers).
    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
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
            .post(format!("{}/articles", self.api_base))
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
            .get(format!("{}/users/me", self.api_base))
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
    use crate::Strategy;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/articles"))
            .and(header("api-key", "test-key"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 12345,
                "url": "https://dev.to/testuser/my-article-abc123"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = DevtoStrategy::new(DevtoCredentials {
            api_key: "test-key".to_string(),
        })
        .unwrap()
        .with_api_base(mock_server.uri());

        let result = strategy
            .post("My Article Title\nArticle body here", None)
            .await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, "12345");
        assert_eq!(
            response.url.as_deref(),
            Some("https://dev.to/testuser/my-article-abc123")
        );
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/articles"))
            .respond_with(ResponseTemplate::new(422).set_body_string("Validation failed"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = DevtoStrategy::new(DevtoCredentials {
            api_key: "bad-key".to_string(),
        })
        .unwrap()
        .with_api_base(mock_server.uri());

        let result = strategy.post("Title\nBody", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Dev.to API error"));
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me"))
            .and(header("api-key", "valid-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 1,
                "username": "testuser"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = DevtoStrategy::new(DevtoCredentials {
            api_key: "valid-key".to_string(),
        })
        .unwrap()
        .with_api_base(mock_server.uri());

        assert!(strategy.validate_credentials().await.unwrap());
    }

    #[tokio::test]
    async fn test_validate_credentials_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/users/me"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = DevtoStrategy::new(DevtoCredentials {
            api_key: "bad-key".to_string(),
        })
        .unwrap()
        .with_api_base(mock_server.uri());

        assert!(!strategy.validate_credentials().await.unwrap());
    }
}
