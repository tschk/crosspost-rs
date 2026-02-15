use serde::{Deserialize, Serialize};

/// Image data embedded in a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEmbed {
    /// Raw image bytes
    pub data: Vec<u8>,
    /// Alt text for accessibility
    pub alt: Option<String>,
    /// MIME type (detected automatically if not provided)
    pub mime_type: Option<String>,
}

/// Options for posting a message
#[derive(Debug, Clone, Default)]
pub struct PostOptions {
    /// Images to attach to the post (max 4)
    pub images: Vec<ImageEmbed>,
}

/// An entry for posting different messages to specific strategies
#[derive(Debug, Clone)]
pub struct PostToEntry {
    /// The strategy ID to post to (e.g., "bluesky", "twitter")
    pub strategy_id: String,
    /// The message to post
    pub message: String,
    /// Optional images for this specific post
    pub images: Option<Vec<ImageEmbed>>,
}

/// Result of posting to a single strategy
#[derive(Debug, Clone)]
pub enum PostResult {
    /// Successfully posted
    Success {
        /// Display name of the strategy (e.g., "Bluesky")
        name: String,
        /// The platform-specific post ID
        post_id: String,
        /// URL to the posted content
        url: Option<String>,
    },
    /// Failed to post
    Failure {
        /// Display name of the strategy (e.g., "Bluesky")
        name: String,
        /// Reason for failure
        reason: String,
    },
}

// --- Credential types ---

#[derive(Debug, Clone)]
pub struct TwitterCredentials {
    pub access_token: String,
}

#[derive(Debug, Clone)]
pub struct BlueskyCredentials {
    pub identifier: String,
    pub password: String,
    pub host: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MastodonCredentials {
    pub access_token: String,
    pub host: String,
}

#[derive(Debug, Clone)]
pub struct LinkedInCredentials {
    pub access_token: String,
}

#[derive(Debug, Clone)]
pub struct DiscordCredentials {
    pub bot_token: String,
    pub channel_id: String,
}

#[derive(Debug, Clone)]
pub struct DiscordWebhookCredentials {
    pub webhook_url: String,
}

#[derive(Debug, Clone)]
pub struct TelegramCredentials {
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Clone)]
pub struct DevtoCredentials {
    pub api_key: String,
}

#[derive(Debug, Clone)]
pub struct SlackCredentials {
    pub bot_token: String,
    pub channel: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NostrCredentials {
    pub private_key: String,
    pub relays: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FacebookCredentials {
    pub access_token: String,
}

#[derive(Debug, Clone)]
pub struct InstagramCredentials {
    pub access_token: String,
}

#[derive(Debug, Clone)]
pub struct YouTubeCredentials {
    pub access_token: String,
}

#[derive(Debug, Clone)]
pub struct TikTokCredentials {
    pub access_token: String,
}

#[derive(Debug, Clone)]
pub struct RedditCredentials {
    pub access_token: String,
    pub subreddit: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TwitchCredentials {
    pub access_token: String,
    pub client_id: String,
}
