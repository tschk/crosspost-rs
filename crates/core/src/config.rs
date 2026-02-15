use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub oauth: OAuthConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    pub jwt_secret: String,
    /// Token expiry in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_token_expiry")]
    pub token_expiry_secs: u64,
}

fn default_token_expiry() -> u64 {
    3600
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub base_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub surrealdb_url: String,
    pub rocksdb_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlatformOAuthConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
}

impl AppConfig {
    pub fn from_env() -> crate::Result<Self> {
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            .add_source(config::Environment::default().separator("__"))
            .build()
            .map_err(|e| crate::Error::Config(e.to_string()))?;

        config
            .try_deserialize()
            .map_err(|e| crate::Error::Config(e.to_string()))
    }
}
