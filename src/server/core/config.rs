use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub oauth: OAuthConfig,
    pub auth: AuthConfig,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("server", &self.server)
            .field("database", &self.database)
            .field("auth", &self.auth)
            .field("oauth", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Deserialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    /// Token expiry in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_token_expiry")]
    pub token_expiry_secs: u64,
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("jwt_secret", &"[REDACTED]")
            .field("token_expiry_secs", &self.token_expiry_secs)
            .finish()
    }
}

fn default_token_expiry() -> u64 {
    3600
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    /// Allowed CORS origins. If empty or missing, no wildcard is used (restrictive).
    #[serde(default)]
    pub cors_origins: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub surrealdb_url: String,
    pub rocksdb_path: String,
}

#[derive(Clone, Deserialize)]
pub struct OAuthConfig {
    pub twitter: Option<PlatformOAuthConfig>,
    pub facebook: Option<PlatformOAuthConfig>,
    pub instagram: Option<PlatformOAuthConfig>,
    pub linkedin: Option<PlatformOAuthConfig>,
    pub youtube: Option<PlatformOAuthConfig>,
    pub tiktok: Option<PlatformOAuthConfig>,
    pub reddit: Option<PlatformOAuthConfig>,
    pub twitch: Option<PlatformOAuthConfig>,
    pub slack: Option<PlatformOAuthConfig>,
    pub telegram: Option<PlatformOAuthConfig>,
    pub bluesky: Option<PlatformOAuthConfig>,
    pub mastodon: Option<PlatformOAuthConfig>,
    pub discord: Option<PlatformOAuthConfig>,
    pub discord_webhook: Option<PlatformOAuthConfig>,
    pub devto: Option<PlatformOAuthConfig>,
    pub nostr: Option<PlatformOAuthConfig>,
}

#[derive(Clone, Deserialize)]
pub struct PlatformOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl std::fmt::Debug for PlatformOAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformOAuthConfig")
            .field("client_id", &self.client_id)
            .field("client_secret", &"[REDACTED]")
            .field("redirect_uri", &self.redirect_uri)
            .finish()
    }
}

impl AppConfig {
    pub fn from_env() -> super::Result<Self> {
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()
            .map_err(|e| super::Error::Config(e.to_string()))?;

        config
            .try_deserialize()
            .map_err(|e| super::Error::Config(e.to_string()))
    }
}
