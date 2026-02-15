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

#[async_trait::async_trait]
impl Platform for SlackClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // Slack requires a channel to post to.
        // The channel should be configured per connected account.
        // For now, use the #general channel convention.
        let body = serde_json::json!({
            "channel": "#general",
            "text": request.content,
            "unfurl_links": true,
            "unfurl_media": true,
        });

        let response = self
            .client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(access_token)
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

        let ts = slack_response.ts.unwrap_or_else(|| "unknown".to_string());
        let channel = slack_response
            .channel
            .unwrap_or_else(|| "unknown".to_string());

        Ok(PostResponse {
            platform_post_id: ts.clone(),
            url: Some(format!(
                "https://slack.com/archives/{}/p{}",
                channel,
                ts.replace('.', "")
            )),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let response = self
            .client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(access_token)
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
}
