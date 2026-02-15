use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

pub struct SlackClient {
    client: reqwest::Client,
}

impl SlackClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for SlackClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct SlackPostResponse {
    ok: bool,
    ts: Option<String>,
    channel: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct SlackAuthTestResponse {
    ok: bool,
}

#[derive(Deserialize)]
struct SlackUploadUrlResponse {
    ok: bool,
    upload_url: Option<String>,
    file_id: Option<String>,
    error: Option<String>,
}

impl SlackClient {
    /// Parse access_token as "token|channel_id" or just "token"
    fn parse_token(access_token: &str) -> (&str, &str) {
        if let Some(idx) = access_token.rfind('|') {
            (&access_token[..idx], &access_token[idx + 1..])
        } else {
            (access_token, "#general")
        }
    }

    /// Upload a file to Slack using the 3-step process
    async fn upload_file(
        &self,
        token: &str,
        channel: &str,
        data: &[u8],
        filename: &str,
    ) -> Result<()> {
        // Step 1: Get upload URL
        let url_response = self
            .client
            .get("https://slack.com/api/files.getUploadURLExternal")
            .bearer_auth(token)
            .query(&[("filename", filename), ("length", &data.len().to_string())])
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Slack file upload error: {}", e)))?;

        let url_data: SlackUploadUrlResponse = url_response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Slack response: {}", e)))?;

        if !url_data.ok {
            return Err(Error::Platform(format!(
                "Slack file upload error: {}",
                url_data.error.unwrap_or_else(|| "Unknown".to_string())
            )));
        }

        let upload_url = url_data
            .upload_url
            .ok_or_else(|| Error::Platform("No upload URL returned".to_string()))?;
        let file_id = url_data
            .file_id
            .ok_or_else(|| Error::Platform("No file ID returned".to_string()))?;

        // Step 2: Upload file content
        self.client
            .post(&upload_url)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Slack file upload error: {}", e)))?;

        // Step 3: Complete upload
        let complete_body = serde_json::json!({
            "files": [{"id": file_id}],
            "channel_id": channel,
        });

        self.client
            .post("https://slack.com/api/files.completeUploadExternal")
            .bearer_auth(token)
            .json(&complete_body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Slack file complete error: {}", e)))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl Platform for SlackClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        let (token, channel) = Self::parse_token(access_token);

        // Upload images first if present
        if let Some(ref images) = request.images {
            for (i, img) in images.iter().enumerate() {
                let mime = img
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "image/png".to_string());
                let ext = match mime.as_str() {
                    "image/png" => "png",
                    "image/jpeg" => "jpg",
                    "image/gif" => "gif",
                    _ => "png",
                };
                let filename = format!("image{}.{}", i, ext);
                self.upload_file(token, channel, &img.data, &filename)
                    .await?;
            }
        }

        let body = serde_json::json!({
            "channel": channel,
            "text": request.content,
            "unfurl_links": true,
            "unfurl_media": true,
        });

        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Slack API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Slack API error: {}", error_text)));
        }

        let slack_response: SlackPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Slack response: {}", e)))?;

        if !slack_response.ok {
            return Err(Error::Platform(format!(
                "Slack API error: {}",
                slack_response
                    .error
                    .unwrap_or_else(|| "Unknown".to_string())
            )));
        }

        let ts = slack_response.ts.ok_or_else(|| {
            Error::Platform("Slack API did not return a message timestamp".to_string())
        })?;
        let resp_channel = slack_response
            .channel
            .ok_or_else(|| Error::Platform("Slack API did not return a channel ID".to_string()))?;

        Ok(PostResponse {
            platform_post_id: ts.clone(),
            url: Some(format!(
                "https://slack.com/archives/{}/p{}",
                resp_channel,
                ts.replace('.', "")
            )),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let (token, _) = Self::parse_token(access_token);

        let response = self
            .client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Slack API error: {}", e)))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let result: SlackAuthTestResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Slack response: {}", e)))?;

        Ok(result.ok)
    }

    fn platform_name(&self) -> &'static str {
        "slack"
    }

    fn max_message_length(&self) -> usize {
        40000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token() {
        let (token, channel) = SlackClient::parse_token("xoxb-123|C12345");
        assert_eq!(token, "xoxb-123");
        assert_eq!(channel, "C12345");

        let (token, channel) = SlackClient::parse_token("xoxb-123");
        assert_eq!(token, "xoxb-123");
        assert_eq!(channel, "#general");
    }

    #[test]
    fn test_max_message_length() {
        let client = SlackClient::new();
        assert_eq!(client.max_message_length(), 40000);
    }
}
