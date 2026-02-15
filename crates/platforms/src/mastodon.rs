use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

pub struct MastodonClient {
    client: reqwest::Client,
}

impl MastodonClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for MastodonClient {
    fn default() -> Self {
        Self::new()
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

#[derive(Deserialize)]
struct MastodonAccount {
    #[allow(dead_code)]
    id: String,
}

impl MastodonClient {
    /// Parse access_token as "token|host" where host is the Mastodon instance
    fn parse_token(access_token: &str) -> (&str, &str) {
        if let Some(idx) = access_token.rfind('|') {
            (&access_token[..idx], &access_token[idx + 1..])
        } else {
            (access_token, "mastodon.social")
        }
    }

    /// Upload media to Mastodon
    async fn upload_media(
        &self,
        token: &str,
        host: &str,
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

        let url = format!("https://{}/api/v2/media", host);
        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
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

#[async_trait::async_trait]
impl Platform for MastodonClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        let (token, host) = Self::parse_token(access_token);

        // Upload images if present
        let mut media_ids = Vec::new();
        if let Some(ref images) = request.images {
            for img in images.iter().take(4) {
                let mime = img
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "image/jpeg".to_string());
                let media_id = self
                    .upload_media(token, host, &img.data, &mime, img.alt.as_deref())
                    .await?;
                media_ids.push(media_id);
            }
        }

        let url = format!("https://{}/api/v1/statuses", host);

        let mut body = serde_json::json!({
            "status": request.content,
        });

        if !media_ids.is_empty() {
            body["media_ids"] = serde_json::json!(media_ids);
        }

        let response = self
            .client
            .post(&url)
            .bearer_auth(token)
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
            platform_post_id: status.id,
            url: status.url,
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let (token, host) = Self::parse_token(access_token);
        let url = format!("https://{}/api/v1/accounts/verify_credentials", host);

        let response = self
            .client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Mastodon API error: {}", e)))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let _account: MastodonAccount = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Mastodon response: {}", e)))?;

        Ok(true)
    }

    fn platform_name(&self) -> &'static str {
        "mastodon"
    }

    fn max_message_length(&self) -> usize {
        500
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token() {
        let (token, host) = MastodonClient::parse_token("mytoken|mastodon.social");
        assert_eq!(token, "mytoken");
        assert_eq!(host, "mastodon.social");

        let (token, host) = MastodonClient::parse_token("mytoken");
        assert_eq!(token, "mytoken");
        assert_eq!(host, "mastodon.social");

        let (token, host) = MastodonClient::parse_token("mytoken|custom.instance.com");
        assert_eq!(token, "mytoken");
        assert_eq!(host, "custom.instance.com");
    }

    #[test]
    fn test_max_message_length() {
        let client = MastodonClient::new();
        assert_eq!(client.max_message_length(), 500);
    }
}
