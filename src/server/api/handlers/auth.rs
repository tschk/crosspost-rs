use crate::server::api::{middleware::AppError, state::AppState};
use crate::server::auth::Claims;
use crate::server::core::{
    ConnectedAccount, DirectConnectRequest, Error, OAuthAuthorizationResponse, OAuthCallbackQuery,
    Platform,
};
use crate::{
    BlueskyCredentials, BlueskyStrategy, DevtoCredentials, DevtoStrategy, DiscordCredentials,
    DiscordStrategy, DiscordWebhookCredentials, DiscordWebhookStrategy, NostrCredentials,
    NostrStrategy, Strategy, TelegramCredentials, TelegramStrategy,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use oauth2::TokenResponse;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;
use validator::Validate;

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

    // Get authorization URL (with optional PKCE)
    let (auth_url, state_token, pkce_verifier) = state
        .oauth_handler
        .get_authorization_url(&oauth_client, platform)?;

    // Store state token with user context: platform|user_id|tenant_id (10 minute TTL)
    let cache_key = format!("oauth_state:{}", state_token);
    let cache_value = format!("{}|{}|{}", platform_str, claims.sub, claims.tenant_id);
    state
        .cache
        .store_with_ttl(&cache_key, &cache_value, 600)
        .await?;

    // Store PKCE verifier if present
    if let Some(verifier) = pkce_verifier {
        let pkce_key = format!("oauth_pkce:{}", state_token);
        state
            .cache
            .store_with_ttl(&pkce_key, verifier.secret(), 600)
            .await?;
    }

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

    // Retrieve PKCE verifier if present
    let pkce_key = format!("oauth_pkce:{}", query.state);
    let pkce_verifier = state
        .cache
        .get_token(&pkce_key)
        .await?
        .map(oauth2::PkceCodeVerifier::new);

    // Clean up state token and PKCE verifier
    state.cache.delete_token(&cache_key).await?;
    state.cache.delete_token(&pkce_key).await?;

    // Get OAuth config
    let oauth_config = get_platform_oauth_config(&state, platform)?;

    // Create OAuth client
    let oauth_client = state.oauth_handler.create_oauth_client(
        platform,
        &oauth_config.client_id,
        &oauth_config.client_secret,
        &oauth_config.redirect_uri,
    )?;

    // Exchange code for token (with PKCE verifier if present)
    let token_response = state
        .oauth_handler
        .exchange_code(&oauth_client, query.code, pkce_verifier)
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

    // Fetch real platform account info using access token
    let (platform_account_id, platform_account_name) =
        fetch_platform_account_info(platform, &access_token)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to fetch platform account info for {}: {}",
                    platform,
                    e
                );
                (
                    format!("{}_{}", platform_str, user_id),
                    format!("{} account", platform_str),
                )
            });

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

    Ok((StatusCode::CREATED, Json(account)))
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

/// Directly connect a non-OAuth platform account (Bluesky, Telegram, Nostr, Dev.to, Discord Webhook)
pub async fn connect_direct(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<DirectConnectRequest>,
) -> Result<impl IntoResponse, AppError> {
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| Error::Validation(e.to_string()))?;

    let platform = Platform::from_str(&request.platform).map_err(Error::InvalidRequest)?;

    // Validate token by instantiating the strategy and checking credentials
    let valid = match platform {
        Platform::Bluesky => {
            // Bluesky stores identifier|password in access_token
            let parts: Vec<&str> = request.access_token.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err(Error::Validation(
                    "Bluesky requires identifier|password format".to_string(),
                )
                .into());
            }
            let strategy = BlueskyStrategy::new(BlueskyCredentials {
                identifier: parts[0].to_string(),
                password: parts[1].to_string(),
                host: None,
            })
            .map_err(|e| Error::Validation(e.to_string()))?;
            strategy
                .validate_credentials()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
        }
        Platform::Telegram => {
            // Telegram stores bot_token|chat_id in access_token
            let parts: Vec<&str> = request.access_token.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err(Error::Validation(
                    "Telegram requires bot_token|chat_id format".to_string(),
                )
                .into());
            }
            let strategy = TelegramStrategy::new(TelegramCredentials {
                bot_token: parts[0].to_string(),
                chat_id: parts[1].to_string(),
            })
            .map_err(|e| Error::Validation(e.to_string()))?;
            strategy
                .validate_credentials()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
        }
        Platform::Nostr => {
            // Nostr stores private_key|relay1,relay2 in access_token
            let parts: Vec<&str> = request.access_token.splitn(2, '|').collect();
            let private_key = parts[0].to_string();
            let relays = if parts.len() > 1 {
                parts[1].split(',').map(|s| s.to_string()).collect()
            } else {
                vec!["wss://relay.damus.io".to_string()]
            };
            let strategy = NostrStrategy::new(NostrCredentials {
                private_key,
                relays,
            })
            .map_err(|e| Error::Validation(e.to_string()))?;
            strategy
                .validate_credentials()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
        }
        Platform::Devto => {
            let strategy = DevtoStrategy::new(DevtoCredentials {
                api_key: request.access_token.clone(),
            })
            .map_err(|e| Error::Validation(e.to_string()))?;
            strategy
                .validate_credentials()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
        }
        Platform::DiscordWebhook => {
            let strategy = DiscordWebhookStrategy::new(DiscordWebhookCredentials {
                webhook_url: request.access_token.clone(),
            })
            .map_err(|e| Error::Validation(e.to_string()))?;
            strategy
                .validate_credentials()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
        }
        Platform::Discord => {
            // Discord bot stores bot_token|channel_id in access_token
            let parts: Vec<&str> = request.access_token.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err(Error::Validation(
                    "Discord requires bot_token|channel_id format".to_string(),
                )
                .into());
            }
            let strategy = DiscordStrategy::new(DiscordCredentials {
                bot_token: parts[0].to_string(),
                channel_id: parts[1].to_string(),
            })
            .map_err(|e| Error::Validation(e.to_string()))?;
            strategy
                .validate_credentials()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
        }
        _ => {
            return Err(Error::Validation(format!(
                "Platform {} requires OAuth authentication, not direct connect",
                platform
            ))
            .into());
        }
    };

    if !valid {
        return Err(Error::Validation("Invalid credentials for platform".to_string()).into());
    }

    let account_name = request
        .account_name
        .unwrap_or_else(|| format!("{} account", platform));

    let account = ConnectedAccount {
        id: Uuid::new_v4(),
        user_id: claims.sub,
        tenant_id: claims.tenant_id,
        platform,
        platform_account_id: format!("direct_{}_{}", platform, claims.sub),
        platform_account_name: account_name,
        access_token: request.access_token,
        refresh_token: None,
        token_expires_at: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let account = state.db.create_connected_account(account).await?;

    Ok((StatusCode::CREATED, Json(account)))
}

/// Helper to get platform OAuth config from AppState
pub(crate) fn get_platform_oauth_config(
    state: &AppState,
    platform: Platform,
) -> Result<&crate::server::core::config::PlatformOAuthConfig, Error> {
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
        Platform::Bluesky => state.config.oauth.bluesky.as_ref(),
        Platform::Mastodon => state.config.oauth.mastodon.as_ref(),
        Platform::Discord | Platform::DiscordWebhook => state.config.oauth.discord.as_ref(),
        Platform::Devto => state.config.oauth.devto.as_ref(),
        Platform::Nostr => state.config.oauth.nostr.as_ref(),
    }
    .ok_or_else(|| Error::Config(format!("{} OAuth not configured", platform)))
}

/// Fetch the real account ID and display name from a platform's user info API
async fn fetch_platform_account_info(
    platform: Platform,
    access_token: &str,
) -> Result<(String, String), Error> {
    let client = reqwest::Client::new();

    match platform {
        Platform::Twitter => {
            let resp: serde_json::Value = client
                .get("https://api.twitter.com/2/users/me")
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let data = resp
                .get("data")
                .ok_or_else(|| Error::Platform("Missing data".into()))?;
            let id = data["id"].as_str().unwrap_or("unknown").to_string();
            let name = data["name"]
                .as_str()
                .unwrap_or(data["username"].as_str().unwrap_or("Twitter User"))
                .to_string();
            Ok((id, name))
        }
        Platform::Facebook => {
            let resp: serde_json::Value = client
                .get("https://graph.facebook.com/v18.0/me")
                .query(&[("access_token", access_token), ("fields", "id,name")])
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let id = resp["id"].as_str().unwrap_or("unknown").to_string();
            let name = resp["name"].as_str().unwrap_or("Facebook User").to_string();
            Ok((id, name))
        }
        Platform::Instagram => {
            let resp: serde_json::Value = client
                .get("https://graph.instagram.com/v18.0/me")
                .query(&[("access_token", access_token), ("fields", "id,username")])
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let id = resp["id"].as_str().unwrap_or("unknown").to_string();
            let name = resp["username"]
                .as_str()
                .unwrap_or("Instagram User")
                .to_string();
            Ok((id, name))
        }
        Platform::LinkedIn => {
            let resp: serde_json::Value = client
                .get("https://api.linkedin.com/v2/userinfo")
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let id = resp["sub"].as_str().unwrap_or("unknown").to_string();
            let name = resp["name"].as_str().unwrap_or("LinkedIn User").to_string();
            Ok((id, name))
        }
        Platform::YouTube => {
            let resp: serde_json::Value = client
                .get("https://www.googleapis.com/youtube/v3/channels")
                .query(&[("part", "snippet"), ("mine", "true")])
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let item = resp["items"].as_array().and_then(|a| a.first());
            let id = item
                .and_then(|i| i["id"].as_str())
                .unwrap_or("unknown")
                .to_string();
            let name = item
                .and_then(|i| i["snippet"]["title"].as_str())
                .unwrap_or("YouTube Channel")
                .to_string();
            Ok((id, name))
        }
        Platform::TikTok => {
            let resp: serde_json::Value = client
                .get("https://open.tiktokapis.com/v2/user/info/")
                .query(&[("fields", "open_id,display_name")])
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let user = &resp["data"]["user"];
            let id = user["open_id"].as_str().unwrap_or("unknown").to_string();
            let name = user["display_name"]
                .as_str()
                .unwrap_or("TikTok User")
                .to_string();
            Ok((id, name))
        }
        Platform::Reddit => {
            let resp: serde_json::Value = client
                .get("https://oauth.reddit.com/api/v1/me")
                .bearer_auth(access_token)
                .header("User-Agent", "crosspost-rs/0.1")
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let id = resp["id"].as_str().unwrap_or("unknown").to_string();
            let name = resp["name"].as_str().unwrap_or("Reddit User").to_string();
            Ok((id, name))
        }
        Platform::Twitch => {
            let resp: serde_json::Value = client
                .get("https://api.twitch.tv/helix/users")
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let user = resp["data"].as_array().and_then(|a| a.first());
            let id = user
                .and_then(|u| u["id"].as_str())
                .unwrap_or("unknown")
                .to_string();
            let name = user
                .and_then(|u| u["display_name"].as_str())
                .unwrap_or("Twitch User")
                .to_string();
            Ok((id, name))
        }
        Platform::Slack => {
            let resp: serde_json::Value = client
                .post("https://slack.com/api/auth.test")
                .bearer_auth(access_token)
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let id = resp["user_id"].as_str().unwrap_or("unknown").to_string();
            let name = resp["user"].as_str().unwrap_or("Slack User").to_string();
            Ok((id, name))
        }
        Platform::Mastodon => {
            let (token, host) = if let Some(idx) = access_token.find('|') {
                (&access_token[..idx], &access_token[idx + 1..])
            } else {
                (access_token, "mastodon.social")
            };
            let resp: serde_json::Value = client
                .get(format!(
                    "https://{}/api/v1/accounts/verify_credentials",
                    host
                ))
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let id = resp["id"].as_str().unwrap_or("unknown").to_string();
            let name = resp["display_name"]
                .as_str()
                .or_else(|| resp["username"].as_str())
                .unwrap_or("Mastodon User")
                .to_string();
            Ok((id, name))
        }
        Platform::Discord => {
            let resp: serde_json::Value = client
                .get("https://discord.com/api/v10/users/@me")
                .header("Authorization", format!("Bot {}", access_token))
                .send()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?
                .json()
                .await
                .map_err(|e| Error::Platform(e.to_string()))?;
            let id = resp["id"].as_str().unwrap_or("unknown").to_string();
            let name = resp["username"]
                .as_str()
                .unwrap_or("Discord User")
                .to_string();
            Ok((id, name))
        }
        _ => Err(Error::Platform(format!(
            "Platform {} does not support OAuth account info fetching",
            platform
        ))),
    }
}
