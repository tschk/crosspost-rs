use crate::state::AppState;
use axum::{
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check
        .route("/health", get(crate::handlers::health::health_check))
        
        // OAuth routes
        .route(
            "/auth/:platform/connect",
            post(crate::handlers::auth::connect_platform),
        )
        .route(
            "/auth/:platform/callback",
            get(crate::handlers::auth::oauth_callback),
        )
        .route(
            "/auth/accounts/:account_id",
            delete(crate::handlers::auth::disconnect_account),
        )
        
        // Account management
        .route("/accounts", get(crate::handlers::accounts::list_accounts))
        
        // Posting
        .route("/post", post(crate::handlers::posts::create_post))
        .route("/posts", get(crate::handlers::posts::list_posts))
        
        // Scheduling
        .route("/schedule", post(crate::handlers::posts::schedule_post))
        
        .with_state(state)
}
