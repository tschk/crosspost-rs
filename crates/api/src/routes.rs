use crate::{
    rate_limit::{create_rate_limiter, create_rate_limiter_per_minute, RateLimitLayer},
    state::AppState,
};
use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post},
    Router,
};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    // Rate limiters for different endpoint groups
    let auth_limiter = create_rate_limiter_per_minute(20); // 20 auth attempts/min
    let post_limiter = create_rate_limiter(5); // 5 posts/sec
    let read_limiter = create_rate_limiter(30); // 30 reads/sec

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(crate::handlers::health::health_check))
        .route("/auth/register", post(crate::handlers::user_auth::register))
        .route("/auth/login", post(crate::handlers::user_auth::login))
        // OAuth callbacks must be public (user redirects back from provider)
        .route(
            "/auth/:platform/callback",
            get(crate::handlers::auth::oauth_callback),
        )
        .layer(RateLimitLayer::new(auth_limiter));

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
        // Read endpoints
        .route("/posts", get(crate::handlers::posts::list_posts))
        .layer(RateLimitLayer::new(read_limiter));

    // Write-heavy protected routes (stricter rate limit)
    let write_routes = Router::new()
        .route("/post", post(crate::handlers::posts::create_post))
        .route("/schedule", post(crate::handlers::posts::schedule_post))
        .layer(RateLimitLayer::new(post_limiter));

    // Combine protected routes with auth middleware
    let authenticated = Router::new()
        .merge(protected_routes)
        .merge(write_routes)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            crate::middleware::auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(authenticated)
        .with_state(state)
}
