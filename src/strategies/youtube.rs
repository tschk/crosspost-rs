use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{PostResponse, Strategy};
use crate::types::{PostOptions, YouTubeCredentials};
use serde::Deserialize;

/// Strategy for posting to YouTube community posts via Data API.
pub struct YouTubeStrategy {
    client: reqwest::Client,
    credentials: YouTubeCredentials,
    api_base: String,
}

impl YouTubeStrategy {
    pub fn new(credentials: YouTubeCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "YouTube access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: "https://www.googleapis.com".to_string(),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(YouTubeCredentials {
            access_token: required_env("YOUTUBE_ACCESS_TOKEN")?,
        })
    }
}

#[derive(Deserialize)]
struct YouTubeChannelListResponse {
    items: Vec<YouTubeChannelItem>,
}

#[derive(Deserialize)]
struct YouTubeChannelItem {
    id: String,
}

#[async_trait::async_trait]
impl Strategy for YouTubeStrategy {
    fn name(&self) -> &str {
        "YouTube"
    }

    fn id(&self) -> &str {
        "youtube"
    }

    fn max_message_length(&self) -> usize {
        5000
    }

    async fn post(&self, message: &str, _options: Option<&PostOptions>) -> Result<PostResponse> {
        // Get channel ID
        let channel_response = self
            .client
            .get(format!("{}/youtube/v3/channels", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .query(&[("part", "id"), ("mine", "true")])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("YouTube API error: {}", e)))?;

        if !channel_response.status().is_success() {
            let error_text = channel_response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "YouTube API error: {}",
                error_text
            )));
        }

        let channels: YouTubeChannelListResponse = channel_response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse YouTube response: {}", e)))?;

        let channel_id = channels
            .items
            .first()
            .map(|c| c.id.clone())
            .ok_or_else(|| Error::Platform("No YouTube channel found".to_string()))?;

        let body = serde_json::json!({
            "snippet": {
                "channelId": channel_id,
                "type": "textPost",
                "textOriginal": message,
            }
        });

        let response = self
            .client
            .post(format!("{}/youtube/v3/activities", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .query(&[("part", "snippet")])
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("YouTube API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "YouTube API error: {}",
                error_text
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse YouTube response: {}", e)))?;

        let post_id = result["id"]
            .as_str()
            .ok_or_else(|| Error::Platform("YouTube API did not return a post ID".to_string()))?
            .to_string();

        Ok(PostResponse {
            id: post_id.clone(),
            url: Some(format!(
                "https://www.youtube.com/channel/{}/community?lb={}",
                channel_id, post_id
            )),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/youtube/v3/channels", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .query(&[("part", "id"), ("mine", "true")])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("YouTube API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> YouTubeStrategy {
        YouTubeStrategy::new(YouTubeCredentials {
            access_token: "test-token".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(YouTubeStrategy::new(YouTubeCredentials {
            access_token: String::new(),
        })
        .is_err());

        assert!(YouTubeStrategy::new(YouTubeCredentials {
            access_token: "token".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = YouTubeStrategy::new(YouTubeCredentials {
            access_token: "t".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "youtube");
        assert_eq!(s.name(), "YouTube");
        assert_eq!(s.max_message_length(), 5000);
    }

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/youtube/v3/channels"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"items": [{"id": "UC123"}]})),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/youtube/v3/activities"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "post456"})),
            )
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.post("Hello YouTube!", None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.id, "post456");
        assert_eq!(
            response.url,
            Some("https://www.youtube.com/channel/UC123/community?lb=post456".to_string())
        );
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/youtube/v3/channels"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"items": [{"id": "UC123"}]})),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/youtube/v3/activities"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.post("Hello YouTube!", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/youtube/v3/channels"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"items": [{"id": "UC123"}]})),
            )
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.validate_credentials().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_validate_credentials_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/youtube/v3/channels"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.validate_credentials().await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
