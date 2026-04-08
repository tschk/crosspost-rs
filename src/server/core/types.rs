use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Represents a tenant (marketing agency client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a user within a tenant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub name: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to register a new user
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1, max = 200))]
    pub name: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
    pub tenant_name: Option<String>,
}

/// Request to log in
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Auth token response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthTokenResponse {
    pub token: String,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
}

/// Social media platform types
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Twitter,
    Facebook,
    Instagram,
    LinkedIn,
    YouTube,
    TikTok,
    Reddit,
    Twitch,
    Slack,
    Telegram,
    Bluesky,
    Mastodon,
    Discord,
    DiscordWebhook,
    Devto,
    Nostr,
}

impl Platform {
    pub fn as_str(&self) -> &str {
        match self {
            Platform::Twitter => "twitter",
            Platform::Facebook => "facebook",
            Platform::Instagram => "instagram",
            Platform::LinkedIn => "linkedin",
            Platform::YouTube => "youtube",
            Platform::TikTok => "tiktok",
            Platform::Reddit => "reddit",
            Platform::Twitch => "twitch",
            Platform::Slack => "slack",
            Platform::Telegram => "telegram",
            Platform::Bluesky => "bluesky",
            Platform::Mastodon => "mastodon",
            Platform::Discord => "discord",
            Platform::DiscordWebhook => "discord_webhook",
            Platform::Devto => "devto",
            Platform::Nostr => "nostr",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "twitter" => Ok(Platform::Twitter),
            "facebook" => Ok(Platform::Facebook),
            "instagram" => Ok(Platform::Instagram),
            "linkedin" => Ok(Platform::LinkedIn),
            "youtube" => Ok(Platform::YouTube),
            "tiktok" => Ok(Platform::TikTok),
            "reddit" => Ok(Platform::Reddit),
            "twitch" => Ok(Platform::Twitch),
            "slack" => Ok(Platform::Slack),
            "telegram" => Ok(Platform::Telegram),
            "bluesky" => Ok(Platform::Bluesky),
            "mastodon" => Ok(Platform::Mastodon),
            "discord" => Ok(Platform::Discord),
            "discord_webhook" | "discordwebhook" => Ok(Platform::DiscordWebhook),
            "devto" | "dev.to" => Ok(Platform::Devto),
            "nostr" => Ok(Platform::Nostr),
            _ => Err(format!("Unknown platform: {}", s)),
        }
    }
}

/// Represents a connected social media account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedAccount {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub platform: Platform,
    pub platform_account_id: String,
    pub platform_account_name: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Post status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PostStatus {
    Pending,
    Success,
    Failed,
    Scheduled,
}

/// Represents a post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub content: String,
    pub status: PostStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a post to a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPost {
    pub id: Uuid,
    pub post_id: Uuid,
    pub account_id: Uuid,
    pub platform: Platform,
    pub platform_post_id: Option<String>,
    pub status: PostStatus,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a scheduled post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledPost {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub content: String,
    pub scheduled_for: DateTime<Utc>,
    pub account_ids: Vec<Uuid>,
    pub status: PostStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Image data for post attachments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    /// URL to fetch the image from, or base64-encoded data
    pub url: Option<String>,
    /// Base64-encoded image data
    pub data: Option<String>,
    /// Alt text for accessibility
    pub alt: Option<String>,
}

/// Request to create a post
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 10000))]
    pub content: String,
    #[validate(length(min = 1))]
    pub account_ids: Vec<Uuid>,
    pub media_urls: Option<Vec<String>>,
    pub images: Option<Vec<ImageData>>,
}

/// Response for a created post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePostResponse {
    pub post_id: Uuid,
    pub results: Vec<PlatformPostResult>,
}

/// Result of posting to a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformPostResult {
    pub account_id: Uuid,
    pub platform: Option<Platform>,
    pub status: PostStatus,
    pub platform_post_id: Option<String>,
    pub error_message: Option<String>,
}

/// Request to schedule a post
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SchedulePostRequest {
    #[validate(length(min = 1, max = 10000))]
    pub content: String,
    #[validate(length(min = 1))]
    pub account_ids: Vec<Uuid>,
    pub scheduled_for: DateTime<Utc>,
    pub media_urls: Option<Vec<String>>,
}

/// Request to update a scheduled post
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateScheduledPostRequest {
    #[validate(length(min = 1, max = 10000))]
    pub content: Option<String>,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub account_ids: Option<Vec<Uuid>>,
}

/// Request to directly connect a non-OAuth platform account
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DirectConnectRequest {
    #[validate(length(min = 1))]
    pub platform: String,
    #[validate(length(min = 1))]
    pub access_token: String,
    pub account_name: Option<String>,
}

/// API-safe response for a connected account (strips tokens)
#[derive(Debug, Clone, Serialize)]
pub struct ConnectedAccountResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    pub platform: Platform,
    pub platform_account_id: String,
    pub platform_account_name: String,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<ConnectedAccount> for ConnectedAccountResponse {
    fn from(a: ConnectedAccount) -> Self {
        Self {
            id: a.id,
            user_id: a.user_id,
            tenant_id: a.tenant_id,
            platform: a.platform,
            platform_account_id: a.platform_account_id,
            platform_account_name: a.platform_account_name,
            token_expires_at: a.token_expires_at,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

impl From<&ConnectedAccount> for ConnectedAccountResponse {
    fn from(a: &ConnectedAccount) -> Self {
        Self {
            id: a.id,
            user_id: a.user_id,
            tenant_id: a.tenant_id,
            platform: a.platform,
            platform_account_id: a.platform_account_id.clone(),
            platform_account_name: a.platform_account_name.clone(),
            token_expires_at: a.token_expires_at,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

/// Pagination query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

/// OAuth authorization URL response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthAuthorizationResponse {
    pub authorization_url: String,
    pub state: String,
}

/// OAuth callback data
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    pub state: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_platform_from_str() {
        assert_eq!(Platform::from_str("twitter").unwrap(), Platform::Twitter);
        assert_eq!(Platform::from_str("TWITTER").unwrap(), Platform::Twitter);
        assert_eq!(Platform::from_str("Facebook").unwrap(), Platform::Facebook);
        assert_eq!(Platform::from_str("linkedin").unwrap(), Platform::LinkedIn);
        assert_eq!(Platform::from_str("youtube").unwrap(), Platform::YouTube);
        assert_eq!(Platform::from_str("tiktok").unwrap(), Platform::TikTok);
        assert_eq!(Platform::from_str("reddit").unwrap(), Platform::Reddit);
        assert_eq!(Platform::from_str("twitch").unwrap(), Platform::Twitch);
        assert_eq!(Platform::from_str("slack").unwrap(), Platform::Slack);
        assert_eq!(Platform::from_str("telegram").unwrap(), Platform::Telegram);
        assert_eq!(Platform::from_str("bluesky").unwrap(), Platform::Bluesky);
        assert_eq!(Platform::from_str("mastodon").unwrap(), Platform::Mastodon);
        assert_eq!(Platform::from_str("discord").unwrap(), Platform::Discord);
        assert_eq!(
            Platform::from_str("discord_webhook").unwrap(),
            Platform::DiscordWebhook
        );
        assert_eq!(
            Platform::from_str("discordwebhook").unwrap(),
            Platform::DiscordWebhook
        );
        assert_eq!(Platform::from_str("devto").unwrap(), Platform::Devto);
        assert_eq!(Platform::from_str("dev.to").unwrap(), Platform::Devto);
        assert_eq!(Platform::from_str("nostr").unwrap(), Platform::Nostr);
        assert!(Platform::from_str("unknown").is_err());
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(Platform::Twitter.to_string(), "twitter");
        assert_eq!(Platform::Facebook.to_string(), "facebook");
        assert_eq!(Platform::YouTube.to_string(), "youtube");
    }

    #[test]
    fn test_platform_as_str() {
        assert_eq!(Platform::Twitter.as_str(), "twitter");
        assert_eq!(Platform::Instagram.as_str(), "instagram");
    }

    #[test]
    fn test_platform_roundtrip() {
        for platform in [
            Platform::Twitter,
            Platform::Facebook,
            Platform::Instagram,
            Platform::LinkedIn,
            Platform::YouTube,
            Platform::TikTok,
            Platform::Reddit,
            Platform::Twitch,
            Platform::Slack,
            Platform::Telegram,
            Platform::Bluesky,
            Platform::Mastodon,
            Platform::Discord,
            Platform::DiscordWebhook,
            Platform::Devto,
            Platform::Nostr,
        ] {
            let s = platform.as_str();
            let parsed = Platform::from_str(s).unwrap();
            assert_eq!(platform, parsed);
        }
    }

    #[test]
    fn test_platform_serde_roundtrip() {
        let platform = Platform::Twitter;
        let json = serde_json::to_string(&platform).unwrap();
        assert_eq!(json, "\"twitter\"");
        let parsed: Platform = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, platform);
    }

    #[test]
    fn test_post_status_serde() {
        let status = PostStatus::Success;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"success\"");
        let parsed: PostStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn test_create_post_request_validation() {
        use validator::Validate;

        let valid = CreatePostRequest {
            content: "Hello world".to_string(),
            account_ids: vec![Uuid::new_v4()],
            media_urls: None,
            images: None,
        };
        assert!(valid.validate().is_ok());

        let empty_content = CreatePostRequest {
            content: String::new(),
            account_ids: vec![Uuid::new_v4()],
            media_urls: None,
            images: None,
        };
        assert!(empty_content.validate().is_err());

        let no_accounts = CreatePostRequest {
            content: "Hello".to_string(),
            account_ids: vec![],
            media_urls: None,
            images: None,
        };
        assert!(no_accounts.validate().is_err());
    }

    #[test]
    fn test_register_request_validation() {
        use validator::Validate;

        let valid = RegisterRequest {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password: "securepassword".to_string(),
            tenant_name: None,
        };
        assert!(valid.validate().is_ok());

        let bad_email = RegisterRequest {
            email: "not-an-email".to_string(),
            name: "Test User".to_string(),
            password: "securepassword".to_string(),
            tenant_name: None,
        };
        assert!(bad_email.validate().is_err());

        let short_password = RegisterRequest {
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            password: "short".to_string(),
            tenant_name: None,
        };
        assert!(short_password.validate().is_err());
    }
}
