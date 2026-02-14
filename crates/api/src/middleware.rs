use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use crosspost_core::Error;

/// Middleware for tenant isolation
pub async fn tenant_isolation(
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract tenant ID from headers or JWT token
    // This is a placeholder implementation
    let _tenant_id = request
        .headers()
        .get("X-Tenant-ID")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| Error::Unauthorized("Missing tenant ID".to_string()))?;

    // TODO: Validate tenant ID and attach to request extensions
    
    Ok(next.run(request).await)
}

/// Application error wrapper for Axum
pub struct AppError(pub Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code = StatusCode::from_u16(self.0.status_code())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

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
