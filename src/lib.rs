//! # Crosspost
//!
//! Cross-post messages to multiple social media platforms concurrently.
//!
//! This library provides a `Client` that orchestrates posting to multiple platforms
//! using the Strategy pattern. Each platform has its own strategy that encapsulates
//! credentials and API logic.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use crosspost::{Client, BlueskyStrategy, BlueskyCredentials, PostResult};
//!
//! # async fn example() -> crosspost::Result<()> {
//! let client = Client::new(vec![
//!     Box::new(BlueskyStrategy::new(BlueskyCredentials {
//!         identifier: "user.bsky.social".into(),
//!         password: "app-password".into(),
//!         host: None,
//!     })?),
//! ]);
//!
//! let results = client.post("Hello from Rust!", None).await;
//! for result in &results {
//!     match result {
//!         PostResult::Success { name, url, .. } => {
//!             println!("Posted to {}: {}", name, url.as_deref().unwrap_or("ok"));
//!         }
//!         PostResult::Failure { name, reason } => {
//!             println!("Failed on {}: {}", name, reason);
//!         }
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod env;
pub mod error;
pub mod strategies;
pub mod strategy;
pub mod types;
pub mod util;

#[cfg(feature = "server")]
pub mod server;

// Re-export core types at crate root
pub use client::Client;
pub use error::{Error, Result};
pub use strategy::{PostResponse, Strategy};
pub use types::{ImageEmbed, PostOptions, PostResult, PostToEntry};

// Re-export all credential types
pub use types::{
    BlueskyCredentials, DevtoCredentials, DiscordCredentials, DiscordWebhookCredentials,
    FacebookCredentials, InstagramCredentials, LinkedInCredentials, MastodonCredentials,
    NostrCredentials, RedditCredentials, SlackCredentials, TelegramCredentials, TikTokCredentials,
    TwitchCredentials, TwitterCredentials, YouTubeCredentials,
};

// Re-export all strategy types
pub use strategies::bluesky::BlueskyStrategy;
pub use strategies::devto::DevtoStrategy;
pub use strategies::discord::DiscordStrategy;
pub use strategies::discord_webhook::DiscordWebhookStrategy;
pub use strategies::facebook::FacebookStrategy;
pub use strategies::instagram::InstagramStrategy;
pub use strategies::linkedin::LinkedInStrategy;
pub use strategies::mastodon::MastodonStrategy;
pub use strategies::nostr::NostrStrategy;
pub use strategies::reddit::RedditStrategy;
pub use strategies::slack::SlackStrategy;
pub use strategies::telegram::TelegramStrategy;
pub use strategies::tiktok::TikTokStrategy;
pub use strategies::twitch::TwitchStrategy;
pub use strategies::twitter::TwitterStrategy;
pub use strategies::youtube::YouTubeStrategy;
