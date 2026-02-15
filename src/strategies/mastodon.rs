use crate::env::{optional_env, required_env};
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{MastodonCredentials, PostOptions};
use serde::Deserialize;

/// Strategy for posting to Mastodon instances.
///
/// Supports configurable instance host and media uploads.
pub struct MastodonStrategy {
    client: reqwest::Client,
    credentials: MastodonCredentials,
}

impl MastodonStrategy {
    /// Create a new Mastodon strategy with the given credentials.
    pub fn new(credentials: MastodonCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "Mastodon access_token is required".to_string(),
            ));
        }
        if credentials.host.is_empty() {
            return Err(Error::Validation("Mastodon host is required".to_string()));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            credentials,
        })
    }

    /// Create a Mastodon strategy from environment variables.
    ///
    /// Reads `MASTODON_ACCESS_TOKEN` and optionally `MASTODON_HOST` (defaults to "mastodon.social").
    pub fn from_env() -> Result<Self> {
        Self::new(MastodonCredentials {
            access_token: required_env("MASTODON_ACCESS_TOKEN")?,
            host: optional_env("MASTODON_HOST").unwrap_or_else(|| "mastodon.social".to_string()),
        })
    }

    async fn upload_media(
        &self,
        data: &[u8],
        mime_type: &str,
        alt: Option<&str>,
    ) -> Result<String> {
        let file_part = reqwest::multipart::Part::bytes(data.to_vec())
            .mime_str(mime_type)
            .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?
            .file_name("image");

        let mut form = reqwest::multipart::Form::new().part("file", file_part);
        if let Some(alt_text) = alt {
            form = form.text("description", alt_text.to_string());
        }

        let url = format!("https://{}/api/v2/media", self.credentials.host);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.credentials.access_token)
            .multipart(form)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Mastodon media upload error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Mastodon media upload failed: {}",
                error_text
            )));
        }

        let media: MastodonMediaAttachment = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse media response: {}", e)))?;

        Ok(media.id)
    }
}

#[derive(Deserialize)]
struct MastodonStatus {
    id: String,
    url: Option<String>,
}

#[derive(Deserialize)]
struct MastodonMediaAttachment {
    id: String,
}

#[async_trait::async_trait]
impl Strategy for MastodonStrategy {
    fn name(&self) -> &str {
        "Mastodon"
    }

    fn id(&self) -> &str {
        "mastodon"
    }

    fn max_message_length(&self) -> usize {
        500
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let images = get_images(options);
        let mut media_ids = Vec::new();
        for img in images.iter().take(4) {
            let mime = img
                .mime_type
                .clone()
                .unwrap_or_else(|| "image/jpeg".to_string());
            let media_id = self
                .upload_media(&img.data, &mime, img.alt.as_deref())
                .await?;
            media_ids.push(media_id);
        }

        let url = format!("https://{}/api/v1/statuses", self.credentials.host);

        let mut body = serde_json::json!({
            "status": message,
        });

        if !media_ids.is_empty() {
            body["media_ids"] = serde_json::json!(media_ids);
        }

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.credentials.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Mastodon API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Mastodon API error: {}",
                error_text
            )));
        }

        let status: MastodonStatus = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Mastodon response: {}", e)))?;

        Ok(PostResponse {
            id: status.id,
            url: status.url,
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let url = format!(
            "https://{}/api/v1/accounts/verify_credentials",
            self.credentials.host
        );

        let response = self
            .client
            .get(&url)
            .bearer_auth(&self.credentials.access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Mastodon API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(MastodonStrategy::new(MastodonCredentials {
            access_token: String::new(),
            host: "mastodon.social".to_string(),
        })
        .is_err());

        assert!(MastodonStrategy::new(MastodonCredentials {
            access_token: "token".to_string(),
            host: String::new(),
        })
        .is_err());

        assert!(MastodonStrategy::new(MastodonCredentials {
            access_token: "token".to_string(),
            host: "mastodon.social".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = MastodonStrategy::new(MastodonCredentials {
            access_token: "t".to_string(),
            host: "mastodon.social".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "mastodon");
        assert_eq!(s.name(), "Mastodon");
        assert_eq!(s.max_message_length(), 500);
    }
}
