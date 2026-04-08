use crate::server::api::{middleware::AppError, state::AppState};
use crate::server::auth::Claims;
use crate::server::core::ConnectedAccountResponse;
use axum::{extract::State, response::IntoResponse, Extension, Json};
use std::sync::Arc;

/// List all connected accounts for the authenticated user
pub async fn list_accounts(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let accounts = state.db.list_connected_accounts_by_user(claims.sub).await?;
    let responses: Vec<ConnectedAccountResponse> = accounts.into_iter().map(Into::into).collect();
    Ok(Json(responses))
}
