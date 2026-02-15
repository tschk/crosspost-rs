use crate::{middleware::AppError, state::AppState};
use axum::{extract::State, response::IntoResponse, Extension, Json};
use crosspost_auth::Claims;
use std::sync::Arc;

/// List all connected accounts for the authenticated user
pub async fn list_accounts(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let accounts = state.db.list_connected_accounts_by_user(claims.sub).await?;
    Ok(Json(accounts))
}
