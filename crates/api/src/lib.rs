pub mod handlers;
pub mod middleware;
pub mod rate_limit;
pub mod routes;
pub mod scheduler;
pub mod state;

use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderValue};
use crosspost_core::{config::AppConfig, Error, Result};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn run_server(config: AppConfig) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Crosspost API server...");

    // Validate JWT secret length
    if config.auth.jwt_secret.len() < 32 {
        return Err(Error::Config(
            "JWT secret must be at least 32 characters".to_string(),
        ));
    }

    // CORS configuration (must be built before config is consumed)
    let cors = build_cors_layer(&config);

    // Initialize state
    let state = state::AppState::new(config).await?;
    let state = Arc::new(state);

    // Create router
    let x_request_id = header::HeaderName::from_static("x-request-id");
    let app = routes::create_router(state.clone())
        .layer(cors)
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000; includeSubDomains"),
        ))
        .layer(PropagateRequestIdLayer::new(x_request_id.clone()))
        .layer(TraceLayer::new_for_http())
        .layer(SetRequestIdLayer::new(x_request_id, MakeRequestUuid))
        .layer(DefaultBodyLimit::max(10 * 1024 * 1024)); // 10MB body limit

    // Start server
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| Error::Internal(format!("Failed to bind to {}: {}", addr, e)))?;

    // Spawn background schedulers
    tokio::spawn(scheduler::run_scheduler(state.clone()));
    tokio::spawn(scheduler::run_token_refresh(state.clone()));

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| Error::Internal(format!("Server error: {}", e)))?;

    Ok(())
}

fn build_cors_layer(config: &AppConfig) -> CorsLayer {
    use tower_http::cors::Any;

    match config.server.cors_origins {
        Some(ref origins) if !origins.is_empty() => {
            let allowed: Vec<axum::http::HeaderValue> =
                origins.iter().filter_map(|o| o.parse().ok()).collect();
            CorsLayer::new()
                .allow_origin(allowed)
                .allow_methods(Any)
                .allow_headers(Any)
        }
        _ => {
            // Restrictive: only same-origin
            CorsLayer::new().allow_methods(Any).allow_headers(Any)
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown");
}
