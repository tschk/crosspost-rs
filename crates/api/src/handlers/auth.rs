use crate::{middleware::AppError, state::AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use crosspost_core::{
    ConnectedAccount, Error, OAuthAuthorizationResponse, OAuthCallbackQuery, Platform,
};
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

/// Initiate OAuth flow for a platform
pub async fn connect_platform(
    State(state): State<Arc<AppState>>,
    Path(platform_str): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let platform = Platform::from_str(&platform_str)
        .map_err(|e| Error::InvalidRequest(e))?;

    // Get OAuth config for platform
    let oauth_config = match platform {
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
    .ok_or_else(|| Error::Config(format!("{} OAuth not configured", platform)))?;

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

    // Store state token in cache for verification
    let cache_key = format!("oauth_state:{}", state_token);
    state.cache.store_token(&cache_key, &platform_str).await?;

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
    let platform = Platform::from_str(&platform_str)
        .map_err(|e| Error::InvalidRequest(e))?;

    // Verify state token
    let cache_key = format!("oauth_state:{}", query.state);
    let stored_platform = state
        .cache
        .get_token(&cache_key)
        .await?
        .ok_or_else(|| Error::OAuth("Invalid state parameter".to_string()))?;

    if stored_platform != platform_str {
        return Err(Error::OAuth("Platform mismatch".to_string()).into());
    }

    // Clean up state token
    state.cache.delete_token(&cache_key).await?;

    // Get OAuth config
    let oauth_config = match platform {
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
    .ok_or_else(|| Error::Config(format!("{} OAuth not configured", platform)))?;

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
        .map(|t| t.secret().clone());
    let expires_at = token_response.expires_in().map(|duration| {
        chrono::Utc::now() + chrono::Duration::seconds(duration.as_secs() as i64)
    });

    // TODO: Get actual user_id and tenant_id from authenticated session
    let user_id = Uuid::new_v4(); // Placeholder
    let tenant_id = Uuid::new_v4(); // Placeholder

    // TODO: Fetch platform account info using access token
    let platform_account_id = "placeholder_account_id".to_string();
    let platform_account_name = "placeholder_name".to_string();

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

/// Disconnect a connected account
pub async fn disconnect_account(
    State(state): State<Arc<AppState>>,
    Path(account_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Verify user owns this account
    
    state.db.delete_connected_account(account_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
