use crate::error::Result;
use crate::types::{ImageEmbed, PostOptions};

/// Response from a successful post
#[derive(Debug, Clone)]
pub struct PostResponse {
    /// Platform-specific post ID
    pub id: String,
    /// URL to the posted content
    pub url: Option<String>,
}

/// Trait for platform-specific posting strategies.
///
/// Each strategy encapsulates credentials and posting logic for a single platform.
/// Strategies are constructed with typed credential structs and can post messages
/// independently.
#[async_trait::async_trait]
pub trait Strategy: Send + Sync {
    /// Human-readable display name (e.g., "Bluesky", "X (formerly Twitter)")
    fn name(&self) -> &str;

    /// Machine identifier (e.g., "bluesky", "twitter")
    fn id(&self) -> &str;

    /// Maximum message length for this platform
    fn max_message_length(&self) -> usize;

    /// Calculate the effective message length.
    ///
    /// Some platforms count URLs differently (e.g., Twitter counts all URLs as 23 chars,
    /// Bluesky counts URLs > 27 chars as 27).
    fn calculate_message_length(&self, message: &str) -> usize {
        message.len()
    }

    /// Post a message to this platform
    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse>;

    /// Extract a URL from a post response (convenience method)
    fn get_url_from_response(&self, response: &PostResponse) -> Option<String> {
        response.url.clone()
    }

    /// Validate that the configured credentials are valid
    async fn validate_credentials(&self) -> Result<bool>;
}

/// Helper to extract images from PostOptions
pub fn get_images(options: Option<&PostOptions>) -> &[ImageEmbed] {
    match options {
        Some(opts) if !opts.images.is_empty() => &opts.images,
        _ => &[],
    }
}
