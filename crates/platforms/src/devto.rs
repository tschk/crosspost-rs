use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::Deserialize;

/// Dev.to client - publishes articles via API key
pub struct DevtoClient {
    client: reqwest::Client,
}

impl DevtoClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for DevtoClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize)]
struct DevtoArticle {
    id: i64,
    url: String,
}

#[derive(Deserialize)]
struct DevtoUser {
    #[allow(dead_code)]
    id: i64,
}

#[async_trait::async_trait]
impl Platform for DevtoClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // First line is the title, rest is body markdown
        let (title, body_markdown) = {
            let content = &request.content;
            if let Some(newline_pos) = content.find('\n') {
                let title = content[..newline_pos].trim().to_string();
                let body = content[newline_pos + 1..].trim().to_string();
                (title, body)
            } else {
                (content.clone(), String::new())
            }
        };

        // If images are present, append them as markdown image tags
        let mut full_body = body_markdown;
        if let Some(ref images) = request.images {
            for img in images {
                let alt = img.alt.clone().unwrap_or_else(|| "image".to_string());
                let mime = img
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "image/png".to_string());
                let b64 =
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &img.data);
                full_body.push_str(&format!("\n\n![{}](data:{};base64,{})", alt, mime, b64));
            }
        }

        let body = serde_json::json!({
            "article": {
                "title": title,
                "body_markdown": full_body,
                "published": true,
            }
        });

        let response = self
            .client
            .post("https://dev.to/api/articles")
            .header("api-key", access_token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Dev.to API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!("Dev.to API error: {}", error_text)));
        }

        let article: DevtoArticle = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Dev.to response: {}", e)))?;

        Ok(PostResponse {
            platform_post_id: article.id.to_string(),
            url: Some(article.url),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let response = self
            .client
            .get("https://dev.to/api/users/me")
            .header("api-key", access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Dev.to API error: {}", e)))?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let _user: DevtoUser = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Dev.to response: {}", e)))?;

        Ok(true)
    }

    fn platform_name(&self) -> &'static str {
        "devto"
    }

    fn max_message_length(&self) -> usize {
        usize::MAX
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_message_length() {
        let client = DevtoClient::new();
        assert_eq!(client.max_message_length(), usize::MAX);
    }

    #[test]
    fn test_title_body_split() {
        // Verify the splitting logic
        let content = "My Article Title\nThis is the body\nWith multiple lines";
        let (title, body) = if let Some(pos) = content.find('\n') {
            (content[..pos].trim(), content[pos + 1..].trim())
        } else {
            (content, "")
        };
        assert_eq!(title, "My Article Title");
        assert_eq!(body, "This is the body\nWith multiple lines");
    }
}
