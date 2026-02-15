use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{PostResponse, Strategy};
use crate::types::{PostOptions, TikTokCredentials};
use serde::Deserialize;

/// Strategy for posting to TikTok via the Content Posting API.
pub struct TikTokStrategy {
    client: reqwest::Client,
    credentials: TikTokCredentials,
}

impl TikTokStrategy {
    pub fn new(credentials: TikTokCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "TikTok access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            credentials,
        })
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
            .post("https://open.tiktokapis.com/v2/post/publish/content/init/")
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
            .get("https://open.tiktokapis.com/v2/user/info/")
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
}
