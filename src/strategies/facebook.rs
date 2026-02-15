use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{FacebookCredentials, PostOptions};
use serde::Deserialize;

/// Strategy for posting to Facebook Pages via Graph API.
///
/// Supports single and multi-image posts.
pub struct FacebookStrategy {
    client: reqwest::Client,
    credentials: FacebookCredentials,
}

impl FacebookStrategy {
    pub fn new(credentials: FacebookCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "Facebook access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            credentials,
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(FacebookCredentials {
            access_token: required_env("FACEBOOK_ACCESS_TOKEN")?,
        })
    }

    async fn post_with_images(
        &self,
        caption: &str,
        images: &[crate::types::ImageEmbed],
    ) -> Result<PostResponse> {
        if images.len() == 1 {
            let img = &images[0];
            let mime = img.mime_type.as_deref().unwrap_or("image/jpeg").to_string();
            let part = reqwest::multipart::Part::bytes(img.data.clone())
                .file_name("image")
                .mime_str(&mime)
                .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?;
            let form = reqwest::multipart::Form::new()
                .text("caption", caption.to_string())
                .part("source", part);

            let response = self
                .client
                .post("https://graph.facebook.com/v18.0/me/photos")
                .query(&[("access_token", &self.credentials.access_token)])
                .multipart(form)
                .send()
                .await
                .map_err(|e| Error::Platform(format!("Facebook photo upload error: {}", e)))?;

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(Error::Platform(format!(
                    "Facebook photo upload failed: {}",
                    error_text
                )));
            }

            let fb_response: FacebookPostResponse = response.json().await.map_err(|e| {
                Error::Platform(format!("Failed to parse Facebook response: {}", e))
            })?;

            return Ok(PostResponse {
                id: fb_response.id.clone(),
                url: Some(format!("https://facebook.com/{}", fb_response.id)),
            });
        }

        // Multiple images: upload each as unpublished, then create feed post
        let mut media_ids = Vec::new();
        for img in images.iter().take(4) {
            let mime = img.mime_type.as_deref().unwrap_or("image/jpeg").to_string();
            let part = reqwest::multipart::Part::bytes(img.data.clone())
                .file_name("image")
                .mime_str(&mime)
                .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?;
            let form = reqwest::multipart::Form::new()
                .text("published", "false".to_string())
                .part("source", part);

            let response = self
                .client
                .post("https://graph.facebook.com/v18.0/me/photos")
                .query(&[("access_token", &self.credentials.access_token)])
                .multipart(form)
                .send()
                .await
                .map_err(|e| Error::Platform(format!("Facebook photo upload error: {}", e)))?;

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return Err(Error::Platform(format!(
                    "Facebook photo upload failed: {}",
                    error_text
                )));
            }

            let fb_response: FacebookPostResponse = response.json().await.map_err(|e| {
                Error::Platform(format!("Failed to parse Facebook response: {}", e))
            })?;

            media_ids.push(fb_response.id);
        }

        let mut query_params: Vec<(String, String)> = vec![
            (
                "access_token".to_string(),
                self.credentials.access_token.clone(),
            ),
            ("message".to_string(), caption.to_string()),
        ];
        for (i, media_id) in media_ids.iter().enumerate() {
            query_params.push((
                format!("attached_media[{}]", i),
                format!("{{\"media_fbid\":\"{}\"}}", media_id),
            ));
        }

        let response = self
            .client
            .post("https://graph.facebook.com/v18.0/me/feed")
            .query(&query_params)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Facebook API error: {}",
                error_text
            )));
        }

        let fb_response: FacebookPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Facebook response: {}", e)))?;

        Ok(PostResponse {
            id: fb_response.id.clone(),
            url: Some(format!("https://facebook.com/{}", fb_response.id)),
        })
    }
}

#[derive(Deserialize)]
struct FacebookPostResponse {
    id: String,
}

#[async_trait::async_trait]
impl Strategy for FacebookStrategy {
    fn name(&self) -> &str {
        "Facebook"
    }

    fn id(&self) -> &str {
        "facebook"
    }

    fn max_message_length(&self) -> usize {
        63206
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let images = get_images(options);
        if !images.is_empty() {
            return self.post_with_images(message, images).await;
        }

        let body = serde_json::json!({
            "message": message,
        });

        let response = self
            .client
            .post("https://graph.facebook.com/v18.0/me/feed")
            .query(&[("access_token", &self.credentials.access_token)])
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Facebook API error: {}",
                error_text
            )));
        }

        let fb_response: FacebookPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Facebook response: {}", e)))?;

        Ok(PostResponse {
            id: fb_response.id.clone(),
            url: Some(format!("https://facebook.com/{}", fb_response.id)),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get("https://graph.facebook.com/v18.0/me")
            .query(&[("access_token", &self.credentials.access_token)])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(FacebookStrategy::new(FacebookCredentials {
            access_token: String::new(),
        })
        .is_err());

        assert!(FacebookStrategy::new(FacebookCredentials {
            access_token: "token".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = FacebookStrategy::new(FacebookCredentials {
            access_token: "t".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "facebook");
        assert_eq!(s.name(), "Facebook");
        assert_eq!(s.max_message_length(), 63206);
    }
}
