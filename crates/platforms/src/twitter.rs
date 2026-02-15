use crate::platform_trait::{Platform, PostRequest, PostResponse};
use base64::Engine;
use crosspost_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub struct TwitterClient {
    client: reqwest::Client,
}

impl TwitterClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for TwitterClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct TwitterPostRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    media: Option<TwitterMediaAttachment>,
}

#[derive(Serialize)]
struct TwitterMediaAttachment {
    media_ids: Vec<String>,
}

#[derive(Deserialize)]
struct TwitterPostResponse {
    data: TwitterPostData,
}

#[derive(Deserialize)]
struct TwitterPostData {
    id: String,
}

#[derive(Deserialize)]
struct TwitterMediaUploadResponse {
    media_id_string: String,
}

#[async_trait::async_trait]
impl Platform for TwitterClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // Upload images if present
        let media = if let Some(ref images) = request.images {
            let mut media_ids = Vec::new();
            for img in images.iter().take(4) {
                let media_data = base64::engine::general_purpose::STANDARD.encode(&img.data);
                let form = reqwest::multipart::Form::new().text("media_data", media_data);

                let upload_resp = self
                    .client
                    .post("https://upload.twitter.com/1.1/media/upload.json")
                    .bearer_auth(access_token)
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|e| Error::Platform(format!("Twitter media upload error: {}", e)))?;

                if !upload_resp.status().is_success() {
                    let error_text = upload_resp
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(Error::Platform(format!(
                        "Twitter media upload failed: {}",
                        error_text
                    )));
                }

                let media_resp: TwitterMediaUploadResponse =
                    upload_resp.json().await.map_err(|e| {
                        Error::Platform(format!("Failed to parse media response: {}", e))
                    })?;

                media_ids.push(media_resp.media_id_string);
            }
            if media_ids.is_empty() {
                None
            } else {
                Some(TwitterMediaAttachment { media_ids })
            }
        } else {
            None
        };

        let url = "https://api.twitter.com/2/tweets";

        let body = TwitterPostRequest {
            text: request.content,
            media,
        };

        let response = self
            .client
            .post(url)
            .bearer_auth(access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitter API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Twitter API error: {}",
                error_text
            )));
        }

        let twitter_response: TwitterPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Twitter response: {}", e)))?;

        Ok(PostResponse {
            platform_post_id: twitter_response.data.id.clone(),
            url: Some(format!(
                "https://twitter.com/i/web/status/{}",
                twitter_response.data.id
            )),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let url = "https://api.twitter.com/2/users/me";

        let response = self
            .client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Twitter API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "twitter"
    }

    fn max_message_length(&self) -> usize {
        280
    }

    fn calculate_message_length(&self, content: &str) -> usize {
        // Twitter counts all URLs as 23 characters
        let mut length = 0;
        let mut remaining = content;

        while let Some(url_start) = remaining
            .find("http://")
            .or_else(|| remaining.find("https://"))
        {
            length += remaining[..url_start].len();
            let after_url = &remaining[url_start..];
            let url_end = after_url
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_url.len());
            length += 23; // All URLs count as 23 chars on Twitter
            remaining = &after_url[url_end..];
        }
        length += remaining.len();
        length
    }
}
