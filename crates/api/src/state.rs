use crosspost_auth::{JwtManager, OAuthHandler, TokenManager};
use crosspost_core::{config::AppConfig, Result};
use crosspost_db::{CacheClient, SurrealDbClient};
use std::sync::Arc;

pub struct AppState {
    pub config: AppConfig,
    pub db: Arc<SurrealDbClient>,
    pub cache: Arc<CacheClient>,
    pub oauth_handler: Arc<OAuthHandler>,
    pub token_manager: Arc<TokenManager>,
    pub jwt: Arc<JwtManager>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self> {
        // Initialize databases
        let db = Arc::new(SurrealDbClient::new(&config.database.surrealdb_url).await?);
        db.init().await?;

        // Use the same SurrealDB instance for caching
        let cache = Arc::new(CacheClient::new(db.get_db_handle()).await?);

        // Initialize OAuth handler and token manager
        let oauth_handler = Arc::new(OAuthHandler::new(db.clone()));
        let token_manager = Arc::new(TokenManager::new(db.clone(), oauth_handler.clone()));

        // Initialize JWT manager
        let jwt = Arc::new(JwtManager::new(
            &config.auth.jwt_secret,
            config.auth.token_expiry_secs,
        ));

        Ok(Self {
            config,
            db,
            cache,
            oauth_handler,
            token_manager,
            jwt,
        })
    }
}
