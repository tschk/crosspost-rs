use crate::state::AppState;
use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(crate::handlers::health::health_check))
        .route("/auth/register", post(crate::handlers::user_auth::register))
        .route("/auth/login", post(crate::handlers::user_auth::login))
        // OAuth callbacks must be public (user redirects back from provider)
        .route(
            "/auth/:platform/callback",
            get(crate::handlers::auth::oauth_callback),
        );

    // Protected routes (JWT auth required)
    let protected_routes = Router::new()
        // OAuth connect
        .route(
            "/auth/:platform/connect",
            post(crate::handlers::auth::connect_platform),
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
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(state)
}
