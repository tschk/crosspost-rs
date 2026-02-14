use crosspost_auth::{OAuthHandler, TokenManager};
use crosspost_core::{config::AppConfig, Result};
use crosspost_db::{RocksDbClient, SurrealDbClient};
use std::sync::Arc;

pub struct AppState {
    pub config: AppConfig,
    pub db: Arc<SurrealDbClient>,
    pub cache: Arc<RocksDbClient>,
    pub oauth_handler: Arc<OAuthHandler>,
    pub token_manager: Arc<TokenManager>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Self> {
        // Initialize databases
        let db = Arc::new(SurrealDbClient::new(&config.database.surrealdb_url).await?);
        db.init().await?;

        // Use the same SurrealDB instance for caching
        let cache = Arc::new(RocksDbClient::new(db.get_db_handle()).await?);

        // Initialize OAuth handler and token manager
        let oauth_handler = Arc::new(OAuthHandler::new(db.clone()));
        let token_manager = Arc::new(TokenManager::new(db.clone(), oauth_handler.clone()));

        Ok(Self {
            config,
            db,
            cache,
            oauth_handler,
            token_manager,
        })
    }
}
