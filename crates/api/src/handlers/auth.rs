use crate::{middleware::AppError, state::AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use crosspost_auth::Claims;
use crosspost_core::{
    ConnectedAccount, Error, OAuthAuthorizationResponse, OAuthCallbackQuery, Platform,
};
use oauth2::TokenResponse;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

/// Initiate OAuth flow for a platform (requires authentication)
pub async fn connect_platform(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(platform_str): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let platform = Platform::from_str(&platform_str).map_err(Error::InvalidRequest)?;

    // Get OAuth config for platform
    let oauth_config = get_platform_oauth_config(&state, platform)?;

    // Create OAuth client
    let oauth_client = state.oauth_handler.create_oauth_client(
        platform,
        &oauth_config.client_id,
        &oauth_config.client_secret,
        &oauth_config.redirect_uri,
    )?;

    // Get authorization URL
    let (auth_url, state_token) = state
        .oauth_handler
        .get_authorization_url(&oauth_client, platform)?;

    // Store state token with user context: platform|user_id|tenant_id
    let cache_key = format!("oauth_state:{}", state_token);
    let cache_value = format!("{}|{}|{}", platform_str, claims.sub, claims.tenant_id);
    state.cache.store_token(&cache_key, &cache_value).await?;

    Ok(Json(OAuthAuthorizationResponse {
        authorization_url: auth_url,
        state: state_token,
    }))
}

/// Handle OAuth callback
pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Path(platform_str): Path<String>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    let platform = Platform::from_str(&platform_str).map_err(Error::InvalidRequest)?;

    // Verify state token and extract user context
    let cache_key = format!("oauth_state:{}", query.state);
    let cached_value = state
        .cache
        .get_token(&cache_key)
        .await?
        .ok_or_else(|| Error::OAuth("Invalid or expired state parameter".to_string()))?;

    // Parse cached value: "platform|user_id|tenant_id"
    let parts: Vec<&str> = cached_value.split('|').collect();
    if parts.len() != 3 {
        return Err(Error::OAuth("Corrupted OAuth state".to_string()).into());
    }
    let (stored_platform, user_id_str, tenant_id_str) = (parts[0], parts[1], parts[2]);

    if stored_platform != platform_str {
        return Err(Error::OAuth("Platform mismatch".to_string()).into());
    }

    let user_id = Uuid::parse_str(user_id_str)
        .map_err(|_| Error::OAuth("Invalid user ID in state".to_string()))?;
    let tenant_id = Uuid::parse_str(tenant_id_str)
        .map_err(|_| Error::OAuth("Invalid tenant ID in state".to_string()))?;

    // Clean up state token
    state.cache.delete_token(&cache_key).await?;

    // Get OAuth config
    let oauth_config = get_platform_oauth_config(&state, platform)?;

    // Create OAuth client
    let oauth_client = state.oauth_handler.create_oauth_client(
        platform,
        &oauth_config.client_id,
        &oauth_config.client_secret,
        &oauth_config.redirect_uri,
    )?;

    // Exchange code for token
    let token_response = state
        .oauth_handler
        .exchange_code(&oauth_client, query.code)
        .await?;

    let access_token = token_response.access_token().secret().clone();
    let refresh_token = token_response
        .refresh_token()
        .map(|t: &oauth2::RefreshToken| t.secret().clone());
    let expires_at = token_response
        .expires_in()
        .map(|duration: std::time::Duration| {
            chrono::Utc::now() + chrono::Duration::seconds(duration.as_secs() as i64)
        });

    // TODO: Fetch platform account info using access token (platform-specific API calls)
    let platform_account_id = format!("{}_{}", platform_str, user_id);
    let platform_account_name = format!("{} account", platform_str);

    // Create connected account
    let account = ConnectedAccount {
        id: Uuid::new_v4(),
        user_id,
        tenant_id,
        platform,
        platform_account_id,
        platform_account_name,
        access_token,
        refresh_token,
        token_expires_at: expires_at,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let account = state.db.create_connected_account(account).await?;

    Ok((StatusCode::OK, Json(account)))
}

/// Disconnect a connected account (requires ownership)
pub async fn disconnect_account(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Path(account_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // Verify user owns this account
    let account = state
        .db
        .get_connected_account(account_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Account {} not found", account_id)))?;

    if account.user_id != claims.sub || account.tenant_id != claims.tenant_id {
        return Err(Error::Forbidden("You do not own this account".to_string()).into());
    }

    state.db.delete_connected_account(account_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Helper to get platform OAuth config from AppState
pub(crate) fn get_platform_oauth_config(
    state: &AppState,
    platform: Platform,
) -> Result<&crosspost_core::config::PlatformOAuthConfig, Error> {
    match platform {
        Platform::Twitter => state.config.oauth.twitter.as_ref(),
        Platform::Facebook => state.config.oauth.facebook.as_ref(),
        Platform::Instagram => state.config.oauth.instagram.as_ref(),
        Platform::LinkedIn => state.config.oauth.linkedin.as_ref(),
        Platform::YouTube => state.config.oauth.youtube.as_ref(),
        Platform::TikTok => state.config.oauth.tiktok.as_ref(),
        Platform::Reddit => state.config.oauth.reddit.as_ref(),
        Platform::Twitch => state.config.oauth.twitch.as_ref(),
        Platform::Slack => state.config.oauth.slack.as_ref(),
        Platform::Telegram => state.config.oauth.telegram.as_ref(),
    }
    .ok_or_else(|| Error::Config(format!("{} OAuth not configured", platform)))
}
