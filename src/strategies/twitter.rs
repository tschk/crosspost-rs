use crate::env::optional_env;
use crate::error::{platform_response_error, Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{PostOptions, TwitterCredentials};
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Strategy for posting to Twitter/X via the v2 API.
///
/// Uses OAuth2 bearer token authentication. URLs in messages are counted as 23 characters.
pub struct TwitterStrategy {
    client: reqwest::Client,
    credentials: TwitterCredentials,
    api_base: String,
    upload_base: String,
}

impl TwitterStrategy {
    /// Create a new Twitter strategy with the given credentials.
    pub fn new(credentials: TwitterCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "Twitter access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: "https://api.twitter.com".to_string(),
            upload_base: "https://upload.twitter.com".to_string(),
        })
    }

    /// Override the API base URL (for testing with mock servers).
    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.upload_base = base.clone();
        self.api_base = base;
        self
    }

    /// Create a Twitter strategy from environment variables.
    ///
    /// Reads `TWITTER_ACCESS_TOKEN` (or `TWITTER_ACCESS_TOKEN_KEY`).
    pub fn from_env() -> Result<Self> {
        Self::new(TwitterCredentials {
            access_token: optional_env("TWITTER_ACCESS_TOKEN")
                .or_else(|| optional_env("TWITTER_ACCESS_TOKEN_KEY"))
                .ok_or_else(|| {
                    Error::Config("Missing TWITTER_ACCESS_TOKEN environment variable".to_string())
                })?,
        })
    }
}

#[derive(Serialize)]
struct TwitterPostRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    media: Option<TwitterMediaAttachment>,
}

#[derive(Serialize)]
struct TwitterMediaAttachment {
    media_ids: Vec<String>,
}

#[derive(Deserialize)]
struct TwitterPostResponse {
    data: TwitterPostData,
}

#[derive(Deserialize)]
struct TwitterPostData {
    id: String,
}

#[derive(Deserialize)]
struct TwitterMediaUploadResponse {
    media_id_string: String,
}

#[async_trait::async_trait]
impl Strategy for TwitterStrategy {
    fn name(&self) -> &str {
        "X (formerly Twitter)"
    }

    fn id(&self) -> &str {
        "twitter"
    }

    fn max_message_length(&self) -> usize {
        280
    }

    fn calculate_message_length(&self, message: &str) -> usize {
        // Twitter counts all URLs as 23 characters
        let mut length = 0;
        let mut remaining = message;

        while let Some(url_start) = remaining
            .find("http://")
            .or_else(|| remaining.find("https://"))
        {
            length += remaining[..url_start].len();
            let after_url = &remaining[url_start..];
            let url_end = after_url
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_url.len());
            length += 23;
            remaining = &after_url[url_end..];
        }
        length += remaining.len();
        length
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let images = get_images(options);

        let media = if !images.is_empty() {
            let mut media_ids = Vec::new();
            for img in images.iter().take(4) {
                let media_data = base64::engine::general_purpose::STANDARD.encode(&img.data);
                let form = reqwest::multipart::Form::new().text("media_data", media_data);

                let upload_resp = self
                    .client
                    .post(format!("{}/1.1/media/upload.json", self.upload_base))
                    .bearer_auth(&self.credentials.access_token)
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| Error::Platform(format!("Twitter media upload error: {}", e)))?;

                if !upload_resp.status().is_success() {
                    return Err(platform_response_error(self.name(), upload_resp).await);
                }

                let media_resp: TwitterMediaUploadResponse =
                    upload_resp.json().await.map_err(|e| {
                        Error::Platform(format!("Failed to parse media response: {}", e))
                    })?;

                media_ids.push(media_resp.media_id_string);
            }
            if media_ids.is_empty() {
                None
            } else {
                Some(TwitterMediaAttachment { media_ids })
            }
        } else {
            None
        };

        let body = TwitterPostRequest {
            text: message.to_string(),
            media,
        };

        let response = self
            .client
            .post(format!("{}/2/tweets", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitter API error: {}", e)))?;

        if !response.status().is_success() {
            return Err(platform_response_error(self.name(), response).await);
        }

        let twitter_response: TwitterPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Twitter response: {}", e)))?;

        Ok(PostResponse {
            id: twitter_response.data.id.clone(),
            url: Some(format!(
                "https://twitter.com/i/web/status/{}",
                twitter_response.data.id
            )),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/2/users/me", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitter API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> TwitterStrategy {
        TwitterStrategy::new(TwitterCredentials {
            access_token: "test-token".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(TwitterStrategy::new(TwitterCredentials {
            access_token: String::new(),
        })
        .is_err());

        assert!(TwitterStrategy::new(TwitterCredentials {
            access_token: "valid-token".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_calculate_message_length() {
        let strategy = test_strategy();

        assert_eq!(strategy.calculate_message_length("hello"), 5);
        // URL counts as 23 chars
        assert_eq!(
            strategy.calculate_message_length("check https://example.com/very/long/url out"),
            // "check " (6) + 23 + " out" (4) = 33
            33
        );
    }

    #[test]
    fn test_strategy_metadata() {
        let strategy = test_strategy();

        assert_eq!(strategy.id(), "twitter");
        assert_eq!(strategy.name(), "X (formerly Twitter)");
        assert_eq!(strategy.max_message_length(), 280);
    }

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/2/tweets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"id": "123"}
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());

        let result = strategy.post("Hello Twitter!", None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, "123");
        assert_eq!(
            response.url.as_deref(),
            Some("https://twitter.com/i/web/status/123")
        );
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/2/tweets"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());

        let result = strategy.post("Hello!", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP 403"));
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/2/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {"id": "1", "username": "testuser"}
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(strategy.validate_credentials().await.unwrap());
    }

    #[tokio::test]
    async fn test_validate_credentials_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/2/users/me"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(!strategy.validate_credentials().await.unwrap());
    }
}
