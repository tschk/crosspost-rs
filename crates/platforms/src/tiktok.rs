use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

pub struct TikTokClient {
    client: reqwest::Client,
}

impl TikTokClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for TikTokClient {
    fn default() -> Self {
        Self::new()
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
impl Platform for TikTokClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // TikTok's Content Posting API for text/photo posts
        let body = serde_json::json!({
            "post_info": {
                "title": request.content,
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
            .bearer_auth(access_token)
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
            platform_post_id: tiktok_response.data.publish_id.clone(),
            url: None, // TikTok doesn't return a direct URL from the publish API
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let response = self
            .client
            .get("https://open.tiktokapis.com/v2/user/info/")
            .bearer_auth(access_token)
            .query(&[("fields", "open_id")])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("TikTok API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "tiktok"
    }

    fn max_message_length(&self) -> usize {
        2200
    }
}
