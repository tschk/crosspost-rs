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
        })
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
                .get("https://oauth.reddit.com/api/v1/me")
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
            .post("https://oauth.reddit.com/api/submit")
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
            .get("https://oauth.reddit.com/api/v1/me")
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
}
