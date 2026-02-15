use crate::env::{optional_env, required_env};
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{PostOptions, SlackCredentials};
use serde::Deserialize;

/// Strategy for posting to Slack channels via Bot API.
///
/// Supports file uploads via 3-step process. Channel defaults to "#general".
pub struct SlackStrategy {
    client: reqwest::Client,
    credentials: SlackCredentials,
}

impl SlackStrategy {
    pub fn new(credentials: SlackCredentials) -> Result<Self> {
        if credentials.bot_token.is_empty() {
            return Err(Error::Validation("Slack bot_token is required".to_string()));
        }
        Ok(Self {
            client: reqwest::Client::new(),
            credentials,
        })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(SlackCredentials {
            bot_token: required_env("SLACK_TOKEN")?,
            channel: optional_env("SLACK_CHANNEL"),
        })
    }

    fn channel(&self) -> &str {
        self.credentials.channel.as_deref().unwrap_or("#general")
    }

    async fn upload_file(&self, channel: &str, data: &[u8], filename: &str) -> Result<()> {
        // Step 1: Get upload URL
        let url_response = self
            .client
            .get("https://slack.com/api/files.getUploadURLExternal")
            .bearer_auth(&self.credentials.bot_token)
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
            .bearer_auth(&self.credentials.bot_token)
            .json(&complete_body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Slack file complete error: {}", e)))?;

        Ok(())
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
struct SlackUploadUrlResponse {
    ok: bool,
    upload_url: Option<String>,
    file_id: Option<String>,
    error: Option<String>,
}

#[async_trait::async_trait]
impl Strategy for SlackStrategy {
    fn name(&self) -> &str {
        "Slack"
    }

    fn id(&self) -> &str {
        "slack"
    }

    fn max_message_length(&self) -> usize {
        40000
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let channel = self.channel();

        let images = get_images(options);
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
            self.upload_file(channel, &img.data, &filename).await?;
        }

        let body = serde_json::json!({
            "channel": channel,
            "text": message,
            "unfurl_links": true,
            "unfurl_media": true,
        });

        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.credentials.bot_token)
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
            id: ts.clone(),
            url: Some(format!(
                "https://slack.com/archives/{}/p{}",
                resp_channel,
                ts.replace('.', "")
            )),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&self.credentials.bot_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Slack API error: {}", e)))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        #[derive(Deserialize)]
        struct AuthTestResponse {
            ok: bool,
        }

        let result: AuthTestResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Slack response: {}", e)))?;

        Ok(result.ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(SlackStrategy::new(SlackCredentials {
            bot_token: String::new(),
            channel: None,
        })
        .is_err());

        assert!(SlackStrategy::new(SlackCredentials {
            bot_token: "xoxb-123".to_string(),
            channel: None,
        })
        .is_ok());
    }

    #[test]
    fn test_channel_default() {
        let s = SlackStrategy::new(SlackCredentials {
            bot_token: "xoxb-123".to_string(),
            channel: None,
        })
        .unwrap();
        assert_eq!(s.channel(), "#general");

        let s = SlackStrategy::new(SlackCredentials {
            bot_token: "xoxb-123".to_string(),
            channel: Some("C12345".to_string()),
        })
        .unwrap();
        assert_eq!(s.channel(), "C12345");
    }

    #[test]
    fn test_strategy_metadata() {
        let s = SlackStrategy::new(SlackCredentials {
            bot_token: "xoxb-123".to_string(),
            channel: None,
        })
        .unwrap();
        assert_eq!(s.id(), "slack");
        assert_eq!(s.name(), "Slack");
        assert_eq!(s.max_message_length(), 40000);
    }
}
