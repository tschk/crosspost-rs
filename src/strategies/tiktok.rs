use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{PostResponse, Strategy};
use crate::types::{PostOptions, TikTokCredentials};
use serde::Deserialize;

/// Strategy for posting to TikTok via the Content Posting API.
pub struct TikTokStrategy {
    client: reqwest::Client,
    credentials: TikTokCredentials,
    api_base: String,
}

impl TikTokStrategy {
    pub fn new(credentials: TikTokCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "TikTok access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: "https://open.tiktokapis.com".to_string(),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(TikTokCredentials {
            access_token: required_env("TIKTOK_ACCESS_TOKEN")?,
        })
    }
}

#[derive(Deserialize)]
struct TikTokPostResponse {
    data: TikTokPostData,
}

#[derive(Deserialize)]
struct TikTokPostData {
    publish_id: String,
}

#[async_trait::async_trait]
impl Strategy for TikTokStrategy {
    fn name(&self) -> &str {
        "TikTok"
    }

    fn id(&self) -> &str {
        "tiktok"
    }

    fn max_message_length(&self) -> usize {
        2200
    }

    async fn post(&self, message: &str, _options: Option<&PostOptions>) -> Result<PostResponse> {
        let body = serde_json::json!({
            "post_info": {
                "title": message,
                "privacy_level": "SELF_ONLY",
                "disable_comment": false,
                "disable_duet": false,
                "disable_stitch": false,
            },
            "source_info": {
                "source": "PULL_FROM_URL",
            }
        });

        let response = self
            .client
            .post(format!("{}/v2/post/publish/content/init/", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("TikTok API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("TikTok API error: {}", error_text)));
        }

        let tiktok_response: TikTokPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse TikTok response: {}", e)))?;

        Ok(PostResponse {
            id: tiktok_response.data.publish_id,
            url: None,
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/v2/user/info/", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .query(&[("fields", "open_id")])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("TikTok API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> TikTokStrategy {
        TikTokStrategy::new(TikTokCredentials {
            access_token: "test-token".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(TikTokStrategy::new(TikTokCredentials {
            access_token: String::new(),
        })
        .is_err());

        assert!(TikTokStrategy::new(TikTokCredentials {
            access_token: "token".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = TikTokStrategy::new(TikTokCredentials {
            access_token: "t".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "tiktok");
        assert_eq!(s.name(), "TikTok");
        assert_eq!(s.max_message_length(), 2200);
    }

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v2/post/publish/content/init/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"data":{"publish_id":"pub123"}})),
            )
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let response = strategy.post("Hello TikTok", None).await.unwrap();
        assert_eq!(response.id, "pub123");
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v2/post/publish/content/init/"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.post("Hello TikTok", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/user/info/"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(strategy.validate_credentials().await.unwrap());
    }

    #[tokio::test]
    async fn test_validate_credentials_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/user/info/"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(!strategy.validate_credentials().await.unwrap());
    }
}
