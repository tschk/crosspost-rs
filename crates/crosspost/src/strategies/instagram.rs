use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{PostResponse, Strategy};
use crate::types::{InstagramCredentials, PostOptions};
use serde::Deserialize;

pub struct InstagramStrategy {
    client: reqwest::Client,
    credentials: InstagramCredentials,
}

impl InstagramStrategy {
    pub fn new(credentials: InstagramCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "Instagram access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            credentials,
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(InstagramCredentials {
            access_token: required_env("INSTAGRAM_ACCESS_TOKEN")?,
        })
    }
}

#[derive(Deserialize)]
struct InstagramPostResponse {
    id: String,
}

#[async_trait::async_trait]
impl Strategy for InstagramStrategy {
    fn name(&self) -> &str {
        "Instagram"
    }

    fn id(&self) -> &str {
        "instagram"
    }

    fn max_message_length(&self) -> usize {
        2200
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        if let Some(opts) = options {
            if !opts.images.is_empty() {
                return Err(Error::Platform(
                    "Instagram requires images to be hosted at a public URL. Direct image upload is not supported.".to_string(),
                ));
            }
        }

        let body = serde_json::json!({
            "caption": message,
        });

        let response = self
            .client
            .post("https://graph.facebook.com/v18.0/me/media")
            .query(&[("access_token", &self.credentials.access_token)])
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Instagram API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Instagram API error: {}",
                error_text
            )));
        }

        let ig_response: InstagramPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Instagram response: {}", e)))?;

        Ok(PostResponse {
            id: ig_response.id.clone(),
            url: Some(format!("https://instagram.com/p/{}", ig_response.id)),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get("https://graph.facebook.com/v18.0/me")
            .query(&[("access_token", &self.credentials.access_token)])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Instagram API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(InstagramStrategy::new(InstagramCredentials {
            access_token: String::new(),
        })
        .is_err());

        assert!(InstagramStrategy::new(InstagramCredentials {
            access_token: "token".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = InstagramStrategy::new(InstagramCredentials {
            access_token: "t".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "instagram");
        assert_eq!(s.name(), "Instagram");
        assert_eq!(s.max_message_length(), 2200);
    }
}
