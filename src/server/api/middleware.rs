use crate::server::auth::Claims;
use crate::server::core::Error;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use super::state::AppState;

/// Middleware for JWT authentication and tenant isolation
pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Error::Unauthorized("Missing Authorization header".to_string()))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| Error::Unauthorized("Invalid Authorization header format".to_string()))?;

    let claims = state.jwt.validate_token(token)?;

    // Store claims in request extensions for handlers to access
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Extract authenticated user claims from request extensions
pub fn get_claims(request: &Request) -> Result<&Claims, Error> {
    request
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| Error::Unauthorized("Not authenticated".to_string()))
}

/// Application error wrapper for Axum
pub struct AppError(pub Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code =
            StatusCode::from_u16(self.0.status_code()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        // For 5xx errors, log the full error but return a generic message to the client
        let message = if status_code.is_server_error() {
            tracing::error!(error = %self.0, "Internal server error");
            "Internal server error".to_string()
        } else {
            self.0.to_string()
        };

        let body = serde_json::json!({
            "error": message,
        });

        (status_code, axum::Json(body)).into_response()
    }
}

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        AppError(err)
    }
}
