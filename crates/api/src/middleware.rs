use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use crosspost_auth::Claims;
use crosspost_core::Error;
use std::sync::Arc;

use crate::state::AppState;

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

        let body = serde_json::json!({
            "error": self.0.to_string(),
        });

        (status_code, axum::Json(body)).into_response()
    }
}

impl From<Error> for AppError {
    fn from(err: Error) -> Self {
        AppError(err)
    }
}
