use crate::{middleware::AppError, state::AppState};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use crosspost_auth::{hash_password, verify_password};
use crosspost_core::{AuthTokenResponse, Error, LoginRequest, RegisterRequest, User};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// Register a new user (and optionally a new tenant)
pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| Error::Validation(e.to_string()))?;

    // Check if user already exists
    if let Some(_existing) = state.db.get_user_by_email(&request.email).await? {
        return Err(Error::Validation("Email already registered".to_string()).into());
    }

    // Create tenant (or use default)
    let tenant_name = request
        .tenant_name
        .unwrap_or_else(|| format!("{}'s Team", request.name));
    let tenant = state.db.create_tenant(&tenant_name).await?;

    // Hash password
    let password_hash = hash_password(&request.password)?;

    // Create user
    let user = User {
        id: Uuid::new_v4(),
        tenant_id: tenant.id,
        email: request.email.clone(),
        name: request.name,
        password_hash,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let user = state.db.create_user_record(user).await?;

    // Generate JWT
    let token = state
        .jwt
        .generate_token(user.id, user.tenant_id, &user.email)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthTokenResponse {
            token,
            user_id: user.id,
            tenant_id: user.tenant_id,
        }),
    ))
}

/// Log in an existing user
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = state
        .db
        .get_user_by_email(&request.email)
        .await?
        .ok_or_else(|| Error::Unauthorized("Invalid email or password".to_string()))?;

    let valid = verify_password(&request.password, &user.password_hash)?;
    if !valid {
        return Err(Error::Unauthorized("Invalid email or password".to_string()).into());
    }

    let token = state
        .jwt
        .generate_token(user.id, user.tenant_id, &user.email)?;

    Ok(Json(AuthTokenResponse {
        token,
        user_id: user.id,
        tenant_id: user.tenant_id,
    }))
}
