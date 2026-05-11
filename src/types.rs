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
    /// Public URL to the image (required by platforms like Instagram that don't accept binary uploads)
    pub image_url: Option<String>,
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
//
// All credential structs implement a custom Debug that redacts secrets.
// This prevents accidental leaking of tokens/passwords to logs.

/// Implement Debug for a credential struct, redacting the specified secret fields.
macro_rules! impl_redacted_debug {
    ($ty:ident { secret: [$($secret:ident),+], plain: [$($plain:ident),*] }) => {
        impl std::fmt::Debug for $ty {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut s = f.debug_struct(stringify!($ty));
                $(s.field(stringify!($secret), &"[REDACTED]");)+
                $(s.field(stringify!($plain), &self.$plain);)*
                s.finish()
            }
        }
    };
}

/// Credentials for Twitter/X API (OAuth2 bearer token).
#[derive(Clone)]
pub struct TwitterCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
}
impl_redacted_debug!(TwitterCredentials {
    secret: [access_token],
    plain: []
});

/// Credentials for Bluesky (AT Protocol app password).
#[derive(Clone)]
pub struct BlueskyCredentials {
    /// Bluesky handle or DID (e.g., "user.bsky.social").
    pub identifier: String,
    /// App password (not your account password).
    pub password: String,
    /// PDS host (defaults to "bsky.social").
    pub host: Option<String>,
}
impl_redacted_debug!(BlueskyCredentials {
    secret: [password],
    plain: [identifier, host]
});

/// Credentials for Mastodon.
#[derive(Clone)]
pub struct MastodonCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
    /// Instance host (e.g., "mastodon.social").
    pub host: String,
}
impl_redacted_debug!(MastodonCredentials {
    secret: [access_token],
    plain: [host]
});

/// Credentials for LinkedIn.
#[derive(Clone)]
pub struct LinkedInCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
}
impl_redacted_debug!(LinkedInCredentials {
    secret: [access_token],
    plain: []
});

/// Credentials for Discord bot posting.
#[derive(Clone)]
pub struct DiscordCredentials {
    /// Bot token from Discord Developer Portal.
    pub bot_token: String,
    /// Channel ID to post to.
    pub channel_id: String,
}
impl_redacted_debug!(DiscordCredentials {
    secret: [bot_token],
    plain: [channel_id]
});

/// Credentials for Discord webhook posting.
#[derive(Clone)]
pub struct DiscordWebhookCredentials {
    /// Full webhook URL (e.g. `<https://discord.com/api/webhooks/...>`).
    pub webhook_url: String,
}
impl_redacted_debug!(DiscordWebhookCredentials {
    secret: [webhook_url],
    plain: []
});

/// Credentials for Telegram Bot API.
#[derive(Clone)]
pub struct TelegramCredentials {
    /// Bot token from BotFather.
    pub bot_token: String,
    /// Chat ID to post to.
    pub chat_id: String,
}
impl_redacted_debug!(TelegramCredentials {
    secret: [bot_token],
    plain: [chat_id]
});

/// Credentials for Dev.to API.
#[derive(Clone)]
pub struct DevtoCredentials {
    /// API key from Dev.to settings.
    pub api_key: String,
}
impl_redacted_debug!(DevtoCredentials {
    secret: [api_key],
    plain: []
});

/// Credentials for Slack Bot API.
#[derive(Clone)]
pub struct SlackCredentials {
    /// Bot user OAuth token (xoxb-...).
    pub bot_token: String,
    /// Channel ID or name (defaults to "#general").
    pub channel: Option<String>,
}
impl_redacted_debug!(SlackCredentials {
    secret: [bot_token],
    plain: [channel]
});

/// Credentials for Nostr protocol.
#[cfg(feature = "nostr")]
#[derive(Clone)]
pub struct NostrCredentials {
    /// Private key in hex or bech32 nsec1 format.
    pub private_key: String,
    /// Relay URLs to publish to (e.g., ["wss://relay.damus.io"]).
    pub relays: Vec<String>,
}
#[cfg(feature = "nostr")]
impl_redacted_debug!(NostrCredentials {
    secret: [private_key],
    plain: [relays]
});

/// Credentials for Facebook Graph API.
#[derive(Clone)]
pub struct FacebookCredentials {
    /// Page access token.
    pub access_token: String,
}
impl_redacted_debug!(FacebookCredentials {
    secret: [access_token],
    plain: []
});

/// Credentials for Instagram Graph API.
#[derive(Clone)]
pub struct InstagramCredentials {
    /// Instagram Graph API access token.
    pub access_token: String,
}
impl_redacted_debug!(InstagramCredentials {
    secret: [access_token],
    plain: []
});

/// Credentials for YouTube Data API.
#[derive(Clone)]
pub struct YouTubeCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
}
impl_redacted_debug!(YouTubeCredentials {
    secret: [access_token],
    plain: []
});

/// Credentials for TikTok Content Posting API.
#[derive(Clone)]
pub struct TikTokCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
}
impl_redacted_debug!(TikTokCredentials {
    secret: [access_token],
    plain: []
});

/// Credentials for Reddit API.
#[derive(Clone)]
pub struct RedditCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
    /// Subreddit to post to (defaults to user profile).
    pub subreddit: Option<String>,
}
impl_redacted_debug!(RedditCredentials {
    secret: [access_token],
    plain: [subreddit]
});

/// Credentials for Twitch API.
#[derive(Clone)]
pub struct TwitchCredentials {
    /// OAuth2 bearer token.
    pub access_token: String,
    /// Client ID from Twitch Developer Console.
    pub client_id: String,
}
impl_redacted_debug!(TwitchCredentials {
    secret: [access_token],
    plain: [client_id]
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_redacts_secrets() {
        let creds = TwitterCredentials {
            access_token: "super-secret-token".to_string(),
        };
        let debug_output = format!("{:?}", creds);
        assert!(!debug_output.contains("super-secret-token"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    #[test]
    fn test_debug_shows_non_secret_fields() {
        let creds = BlueskyCredentials {
            identifier: "user.bsky.social".to_string(),
            password: "secret-password".to_string(),
            host: Some("bsky.social".to_string()),
        };
        let debug_output = format!("{:?}", creds);
        assert!(debug_output.contains("user.bsky.social"));
        assert!(!debug_output.contains("secret-password"));
        assert!(debug_output.contains("[REDACTED]"));
        assert!(debug_output.contains("bsky.social"));
    }
}
