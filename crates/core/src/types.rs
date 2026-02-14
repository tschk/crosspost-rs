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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

/// Request to create a post
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 1, max = 10000))]
    pub content: String,
    #[validate(length(min = 1))]
    pub account_ids: Vec<Uuid>,
    pub media_urls: Option<Vec<String>>,
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
    pub platform: Platform,
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
