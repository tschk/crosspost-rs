use crate::{middleware::AppError, state::AppState};
use axum::{extract::State, response::IntoResponse, Json};
use std::sync::Arc;
use uuid::Uuid;

/// List all connected accounts for a user
pub async fn list_accounts(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Get user_id from authenticated session
    let user_id = Uuid::new_v4(); // Placeholder

    let accounts = state.db.list_connected_accounts_by_user(user_id).await?;

    Ok(Json(accounts))
}
