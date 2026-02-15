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

/// Credentials for Twitter/X API (OAuth2 bearer token).
#[derive(Debug, Clone)]
pub struct TwitterCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
}

/// Credentials for Bluesky (AT Protocol app password).
#[derive(Debug, Clone)]
pub struct BlueskyCredentials {
    /// Bluesky handle or DID (e.g., "user.bsky.social").
    pub identifier: String,
    /// App password (not your account password).
    pub password: String,
    /// PDS host (defaults to "bsky.social").
    pub host: Option<String>,
}

/// Credentials for Mastodon.
#[derive(Debug, Clone)]
pub struct MastodonCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
    /// Instance host (e.g., "mastodon.social").
    pub host: String,
}

/// Credentials for LinkedIn.
#[derive(Debug, Clone)]
pub struct LinkedInCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
}

/// Credentials for Discord bot posting.
#[derive(Debug, Clone)]
pub struct DiscordCredentials {
    /// Bot token from Discord Developer Portal.
    pub bot_token: String,
    /// Channel ID to post to.
    pub channel_id: String,
}

/// Credentials for Discord webhook posting.
#[derive(Debug, Clone)]
pub struct DiscordWebhookCredentials {
    /// Full webhook URL (e.g., "https://discord.com/api/webhooks/...").
    pub webhook_url: String,
}

/// Credentials for Telegram Bot API.
#[derive(Debug, Clone)]
pub struct TelegramCredentials {
    /// Bot token from BotFather.
    pub bot_token: String,
    /// Chat ID to post to.
    pub chat_id: String,
}

/// Credentials for Dev.to API.
#[derive(Debug, Clone)]
pub struct DevtoCredentials {
    /// API key from Dev.to settings.
    pub api_key: String,
}

/// Credentials for Slack Bot API.
#[derive(Debug, Clone)]
pub struct SlackCredentials {
    /// Bot user OAuth token (xoxb-...).
    pub bot_token: String,
    /// Channel ID or name (defaults to "#general").
    pub channel: Option<String>,
}

/// Credentials for Nostr protocol.
#[derive(Debug, Clone)]
pub struct NostrCredentials {
    /// Private key in hex or bech32 nsec1 format.
    pub private_key: String,
    /// Relay URLs to publish to (e.g., ["wss://relay.damus.io"]).
    pub relays: Vec<String>,
}

/// Credentials for Facebook Graph API.
#[derive(Debug, Clone)]
pub struct FacebookCredentials {
    /// Page access token.
    pub access_token: String,
}

/// Credentials for Instagram Graph API.
#[derive(Debug, Clone)]
pub struct InstagramCredentials {
    /// Instagram Graph API access token.
    pub access_token: String,
}

/// Credentials for YouTube Data API.
#[derive(Debug, Clone)]
pub struct YouTubeCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
}

/// Credentials for TikTok Content Posting API.
#[derive(Debug, Clone)]
pub struct TikTokCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
}

/// Credentials for Reddit API.
#[derive(Debug, Clone)]
pub struct RedditCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
    /// Subreddit to post to (defaults to user profile).
    pub subreddit: Option<String>,
}

/// Credentials for Twitch API.
#[derive(Debug, Clone)]
pub struct TwitchCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
    /// Client ID from Twitch Developer Console.
    pub client_id: String,
}
