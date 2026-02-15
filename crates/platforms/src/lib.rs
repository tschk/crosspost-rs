pub mod facebook;
pub mod instagram;
pub mod linkedin;
pub mod platform_trait;
pub mod reddit;
pub mod slack;
pub mod telegram;
pub mod tiktok;
pub mod twitch;
pub mod twitter;
pub mod youtube;

pub use platform_trait::{Platform as PlatformClient, PostRequest, PostResponse};
