use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub struct FacebookClient {
    client: reqwest::Client,
}

impl FacebookClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for FacebookClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct FacebookPostRequest {
    message: String,
}

#[derive(Deserialize)]
struct FacebookPostResponse {
    id: String,
}

impl FacebookClient {
    async fn post_with_images(
        &self,
        access_token: &str,
        caption: &str,
        images: &[crate::platform_trait::ImageEmbed],
    ) -> Result<PostResponse> {
        // For a single image, use /me/photos directly
        // For multiple images, upload each as unpublished then create feed post with attached_media
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
                .query(&[("access_token", access_token)])
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
                platform_post_id: fb_response.id.clone(),
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
                .query(&[("access_token", access_token)])
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

        // Create feed post with attached_media
        let mut query_params: Vec<(String, String)> = vec![
            ("access_token".to_string(), access_token.to_string()),
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
            platform_post_id: fb_response.id.clone(),
            url: Some(format!("https://facebook.com/{}", fb_response.id)),
        })
    }
}

#[async_trait::async_trait]
impl Platform for FacebookClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // If images are provided, upload via /me/photos
        if let Some(ref images) = request.images {
            if !images.is_empty() {
                return self
                    .post_with_images(access_token, &request.content, images)
                    .await;
            }
        }

        let url = "https://graph.facebook.com/v18.0/me/feed";

        let body = FacebookPostRequest {
            message: request.content,
        };

        let response = self
            .client
            .post(url)
            .query(&[("access_token", access_token)])
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
            platform_post_id: fb_response.id.clone(),
            url: Some(format!("https://facebook.com/{}", fb_response.id)),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let url = "https://graph.facebook.com/v18.0/me";

        let response = self
            .client
            .get(url)
            .query(&[("access_token", access_token)])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "facebook"
    }

    fn max_message_length(&self) -> usize {
        63206
    }
}
