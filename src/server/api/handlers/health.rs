use crate::server::api::state::AppState;
use crate::server::db::Database;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use std::sync::Arc;

/// Health check endpoint with DB connectivity verification
pub async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_healthy = Database::health_check(state.db.as_ref()).await.is_ok();

    if db_healthy {
        (
            StatusCode::OK,
            Json(json!({
                "status": "healthy",
                "service": "crosspost-api",
                "database": "connected"
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "degraded",
                "service": "crosspost-api",
                "database": "disconnected"
            })),
        )
    }
}
