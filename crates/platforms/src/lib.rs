pub mod bluesky;
pub mod devto;
pub mod discord;
pub mod discord_webhook;
pub mod facebook;
pub mod instagram;
pub mod linkedin;
pub mod mastodon;
pub mod nostr;
pub mod platform_trait;
pub mod reddit;
pub mod slack;
pub mod telegram;
pub mod tiktok;
pub mod twitch;
pub mod twitter;
pub mod util;
pub mod youtube;

pub use platform_trait::{ImageEmbed, Platform as PlatformClient, PostRequest, PostResponse};

// Re-export all client types at the crate root for convenience
pub use bluesky::BlueskyClient;
pub use devto::DevtoClient;
pub use discord::DiscordClient;
pub use discord_webhook::DiscordWebhookClient;
pub use facebook::FacebookClient;
pub use instagram::InstagramClient;
pub use linkedin::LinkedInClient;
pub use mastodon::MastodonClient;
pub use nostr::NostrClient;
pub use reddit::RedditClient;
pub use slack::SlackClient;
pub use telegram::TelegramClient;
pub use tiktok::TikTokClient;
pub use twitch::TwitchClient;
pub use twitter::TwitterClient;
pub use youtube::YouTubeClient;

/// Prelude for convenient imports
///
/// ```rust
/// use crosspost_platforms::prelude::*;
/// ```
pub mod prelude {
    pub use crate::platform_trait::{
        ImageEmbed, Platform as PlatformClient, PostRequest, PostResponse,
    };
    pub use crate::{
        BlueskyClient, DevtoClient, DiscordClient, DiscordWebhookClient, FacebookClient,
        InstagramClient, LinkedInClient, MastodonClient, NostrClient, RedditClient, SlackClient,
        TelegramClient, TikTokClient, TwitchClient, TwitterClient, YouTubeClient,
    };
}
