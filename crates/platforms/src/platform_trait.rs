use crosspost_core::Result;
use serde::{Deserialize, Serialize};

/// Image data embedded in a post request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEmbed {
    pub data: Vec<u8>,
    pub alt: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostRequest {
    pub content: String,
    pub media_urls: Option<Vec<String>>,
    pub images: Option<Vec<ImageEmbed>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostResponse {
    pub platform_post_id: String,
    pub url: Option<String>,
}

/// Trait for platform-specific posting implementations
#[async_trait::async_trait]
pub trait Platform: Send + Sync {
    /// Post content to the platform
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse>;

    /// Validate access token
    async fn validate_token(&self, access_token: &str) -> Result<bool>;

    /// Get platform name
    fn platform_name(&self) -> &'static str;

    /// Maximum message length for this platform
    fn max_message_length(&self) -> usize {
        usize::MAX
    }

    /// Calculate effective message length (some platforms count URLs differently)
    fn calculate_message_length(&self, content: &str) -> usize {
        content.len()
    }
}
