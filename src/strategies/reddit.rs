use crate::env::{optional_env, required_env};
use crate::error::{Error, Result};
use crate::strategy::{PostResponse, Strategy};
use crate::types::{PostOptions, RedditCredentials};
use serde::Deserialize;

/// Strategy for posting to Reddit via the Submission API.
///
/// Posts to a subreddit if specified, otherwise to the user's profile.
pub struct RedditStrategy {
    client: reqwest::Client,
    credentials: RedditCredentials,
    api_base: String,
}

impl RedditStrategy {
    pub fn new(credentials: RedditCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "Reddit access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .user_agent("crosspost-rs/0.1.0")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: "https://oauth.reddit.com".to_string(),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(RedditCredentials {
            access_token: required_env("REDDIT_ACCESS_TOKEN")?,
            subreddit: optional_env("REDDIT_SUBREDDIT"),
        })
    }
}

#[derive(Deserialize)]
struct RedditSubmitResponse {
    json: RedditSubmitJson,
}

#[derive(Deserialize)]
struct RedditSubmitJson {
    data: Option<RedditSubmitData>,
    errors: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RedditSubmitData {
    url: String,
    name: String,
}

#[derive(Deserialize)]
struct RedditMeResponse {
    name: String,
}

#[async_trait::async_trait]
impl Strategy for RedditStrategy {
    fn name(&self) -> &str {
        "Reddit"
    }

    fn id(&self) -> &str {
        "reddit"
    }

    fn max_message_length(&self) -> usize {
        40000
    }

    async fn post(&self, message: &str, _options: Option<&PostOptions>) -> Result<PostResponse> {
        let subreddit = if let Some(ref sub) = self.credentials.subreddit {
            sub.clone()
        } else {
            // Post to user profile
            let me_response = self
                .client
                .get(format!("{}/api/v1/me", self.api_base))
                .bearer_auth(&self.credentials.access_token)
                .send()
                .await
                .map_err(|e| Error::Platform(format!("Reddit API error: {}", e)))?;

            if !me_response.status().is_success() {
                let error_text = me_response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(Error::Platform(format!("Reddit API error: {}", error_text)));
            }

            let me: RedditMeResponse = me_response
                .json()
                .await
                .map_err(|e| Error::Platform(format!("Failed to parse Reddit response: {}", e)))?;

            format!("u_{}", me.name)
        };

        let params = [
            ("sr", subreddit.as_str()),
            ("kind", "self"),
            ("title", message),
            ("text", message),
            ("api_type", "json"),
        ];

        let response = self
            .client
            .post(format!("{}/api/submit", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .form(&params)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Reddit API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Reddit API error: {}", error_text)));
        }

        let submit_response: RedditSubmitResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Reddit response: {}", e)))?;

        if !submit_response.json.errors.is_empty() {
            return Err(Error::Platform(format!(
                "Reddit submission errors: {:?}",
                submit_response.json.errors
            )));
        }

        let data = submit_response
            .json
            .data
            .ok_or_else(|| Error::Platform("Reddit returned no data".to_string()))?;

        Ok(PostResponse {
            id: data.name,
            url: Some(data.url),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/api/v1/me", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Reddit API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> RedditStrategy {
        RedditStrategy::new(RedditCredentials {
            access_token: "test-token".to_string(),
            subreddit: Some("test_sub".to_string()),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(RedditStrategy::new(RedditCredentials {
            access_token: String::new(),
            subreddit: None,
        })
        .is_err());

        assert!(RedditStrategy::new(RedditCredentials {
            access_token: "token".to_string(),
            subreddit: None,
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = RedditStrategy::new(RedditCredentials {
            access_token: "t".to_string(),
            subreddit: None,
        })
        .unwrap();
        assert_eq!(s.id(), "reddit");
        assert_eq!(s.name(), "Reddit");
        assert_eq!(s.max_message_length(), 40000);
    }

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/submit"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "json": {
                    "data": {
                        "url": "https://reddit.com/r/test/123",
                        "name": "t3_abc"
                    },
                    "errors": []
                }
            })))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.post("Hello Reddit!", None).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.id, "t3_abc");
        assert_eq!(
            response.url,
            Some("https://reddit.com/r/test/123".to_string())
        );
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/api/submit"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.post("Hello Reddit!", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/me"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"name": "testuser"})),
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
            .and(path("/api/v1/me"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.validate_credentials().await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }
}
