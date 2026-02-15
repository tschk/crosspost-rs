use super::{
    rate_limit::{
        create_keyed_rate_limiter, create_keyed_rate_limiter_per_minute,
        create_rate_limiter_per_minute, KeyedRateLimitLayer, RateLimitLayer,
    },
    state::AppState,
};
use axum::{
    middleware as axum_middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;

pub fn create_router(state: Arc<AppState>) -> Router {
    // Rate limiters for different endpoint groups
    let auth_limiter = create_rate_limiter_per_minute(20); // 20 auth attempts/min (global)
    let post_limiter = create_keyed_rate_limiter(5); // 5 posts/sec per user
    let read_limiter = create_keyed_rate_limiter_per_minute(120); // 120 reads/min per user

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(super::handlers::health::health_check))
        .route("/auth/register", post(super::handlers::user_auth::register))
        .route("/auth/login", post(super::handlers::user_auth::login))
        // OAuth callbacks must be public (user redirects back from provider)
        .route(
            "/auth/:platform/callback",
            get(super::handlers::auth::oauth_callback),
        )
        .layer(RateLimitLayer::new(auth_limiter));

    // Protected routes (JWT auth required)
    let protected_routes = Router::new()
        // OAuth connect
        .route(
            "/auth/:platform/connect",
            post(super::handlers::auth::connect_platform),
        )
        .route(
            "/auth/accounts/:account_id",
            delete(super::handlers::auth::disconnect_account),
        )
        // Account management
        .route("/accounts", get(super::handlers::accounts::list_accounts))
        // Read endpoints
        .route("/posts", get(super::handlers::posts::list_posts))
        .route(
            "/schedule",
            get(super::handlers::posts::list_scheduled_posts),
        )
        .layer(KeyedRateLimitLayer::new(read_limiter));

    // Write-heavy protected routes (stricter rate limit, per-user)
    let write_routes = Router::new()
        .route("/post", post(super::handlers::posts::create_post))
        .route("/schedule", post(super::handlers::posts::schedule_post))
        .route(
            "/schedule/:post_id",
            put(super::handlers::posts::update_scheduled_post)
                .delete(super::handlers::posts::cancel_scheduled_post),
        )
        .route(
            "/auth/connect-direct",
            post(super::handlers::auth::connect_direct),
        )
        .layer(KeyedRateLimitLayer::new(post_limiter));

    // Combine protected routes with auth middleware
    let authenticated = Router::new()
        .merge(protected_routes)
        .merge(write_routes)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            super::middleware::auth_middleware,
        ));

    Router::new()
        .merge(public_routes)
        .merge(authenticated)
        .with_state(state)
}
