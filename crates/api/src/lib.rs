pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

use crosspost_core::{config::AppConfig, Error, Result};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn run_server(config: AppConfig) -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Crosspost API server...");

    // Initialize state
    let state = state::AppState::new(config).await?;
    let state = Arc::new(state);

    // Create router
    let app = routes::create_router(state.clone())
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    tracing::info!("Listening on {}", addr);

    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| Error::Internal(format!("Failed to bind to {}: {}", addr, e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| Error::Internal(format!("Server error: {}", e)))?;

    Ok(())
}
