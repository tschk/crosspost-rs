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

// Re-export all client types at the crate root for convenience
pub use facebook::FacebookClient;
pub use instagram::InstagramClient;
pub use linkedin::LinkedInClient;
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
    pub use crate::platform_trait::{Platform as PlatformClient, PostRequest, PostResponse};
    pub use crate::{
        FacebookClient, InstagramClient, LinkedInClient, RedditClient, SlackClient, TelegramClient,
        TikTokClient, TwitchClient, TwitterClient, YouTubeClient,
    };
}
