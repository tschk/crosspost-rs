//! Platform-specific strategy implementations.
//!
//! Each module contains a strategy struct that implements the [`Strategy`](crate::Strategy) trait.

pub mod bluesky;
pub mod devto;
pub mod discord;
pub mod discord_webhook;
pub mod facebook;
pub mod instagram;
pub mod linkedin;
pub mod mastodon;
pub mod nostr;
pub mod reddit;
pub mod slack;
pub mod telegram;
pub mod tiktok;
pub mod twitch;
pub mod twitter;
pub mod youtube;
