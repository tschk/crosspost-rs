use crate::{middleware::AppError, state::AppState};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use crosspost_auth::Claims;
use crosspost_core::{
    CreatePostRequest, CreatePostResponse, Error, PaginationQuery, PlatformPost,
    PlatformPostResult, Post, PostStatus, SchedulePostRequest, ScheduledPost,
    UpdateScheduledPostRequest,
};
use crosspost_platforms::{
    bluesky::BlueskyClient, devto::DevtoClient, discord::DiscordClient,
    discord_webhook::DiscordWebhookClient, facebook::FacebookClient, instagram::InstagramClient,
    linkedin::LinkedInClient, mastodon::MastodonClient, nostr::NostrClient, reddit::RedditClient,
    slack::SlackClient, tiktok::TikTokClient, twitter::TwitterClient, youtube::YouTubeClient,
    PlatformClient, PostRequest,
};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// Create and post content to multiple platforms
pub async fn create_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<CreatePostRequest>,
) -> Result<impl IntoResponse, AppError> {
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| Error::Validation(e.to_string()))?;

    // Validate max 4 images
    if let Some(ref images) = request.images {
        if images.len() > 4 {
            return Err(Error::Validation("Maximum of 4 images per post".to_string()).into());
        }
    }

    let user_id = claims.sub;
    let tenant_id = claims.tenant_id;

    // Create post record
    let post = Post {
        id: Uuid::new_v4(),
        user_id,
        tenant_id,
        content: request.content.clone(),
        status: PostStatus::Pending,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let post = state.db.create_post(post).await?;

    // Post to each connected account
    let mut results = Vec::new();

    for account_id in &request.account_ids {
        let account = state.db.get_connected_account(*account_id).await?;

        let mut account = match account {
            Some(acc) => {
                // Verify account belongs to this user and tenant
                if acc.user_id != user_id || acc.tenant_id != tenant_id {
                    results.push(PlatformPostResult {
                        account_id: *account_id,
                        platform: Some(acc.platform),
                        status: PostStatus::Failed,
                        platform_post_id: None,
                        error_message: Some("Account not found".to_string()),
                    });
                    continue;
                }
                acc
            }
            None => {
                results.push(PlatformPostResult {
                    account_id: *account_id,
                    platform: None,
                    status: PostStatus::Failed,
                    platform_post_id: None,
                    error_message: Some("Account not found".to_string()),
                });
                continue;
            }
        };

        // Attempt token refresh if expired
        if state.token_manager.is_token_expired(&account) {
            let oauth_config =
                crate::handlers::auth::get_platform_oauth_config(&state, account.platform);
            if let Ok(oauth_cfg) = oauth_config {
                if let Ok(oauth_client) = state.oauth_handler.create_oauth_client(
                    account.platform,
                    &oauth_cfg.client_id,
                    &oauth_cfg.client_secret,
                    &oauth_cfg.redirect_uri,
                ) {
                    if let Err(e) = state
                        .token_manager
                        .refresh_token_if_needed(&mut account, &oauth_client)
                        .await
                    {
                        tracing::warn!("Token refresh failed for account {}: {}", account_id, e);
                    }
                }
            }
        }

        // Convert images from CreatePostRequest format to PostRequest format
        let images = if let Some(ref img_data_list) = request.images {
            let mut embeds = Vec::new();
            for img_data in img_data_list {
                if let Some(ref data_b64) = img_data.data {
                    let bytes = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        data_b64,
                    )
                    .map_err(|e| Error::Validation(format!("Invalid base64 image data: {}", e)))?;
                    // Detect MIME and compress
                    let mime = crosspost_platforms::util::images::detect_mime_type(&bytes)
                        .unwrap_or_else(|_| "image/jpeg".to_string());
                    let compressed =
                        crosspost_platforms::util::images::compress_image(&bytes, &mime)
                            .unwrap_or(bytes);
                    embeds.push(crosspost_platforms::ImageEmbed {
                        data: compressed,
                        alt: img_data.alt.clone(),
                        mime_type: Some(mime),
                    });
                }
            }
            if embeds.is_empty() {
                None
            } else {
                Some(embeds)
            }
        } else {
            None
        };

        let post_request = PostRequest {
            content: request.content.clone(),
            media_urls: request.media_urls.clone(),
            images,
        };

        // Message length validation
        let platform_result: crosspost_core::Result<crosspost_platforms::PostResponse> =
            match account.platform {
                crosspost_core::Platform::Twitter => {
                    let client = TwitterClient::new();
                    let msg_len = client.calculate_message_length(&post_request.content);
                    if msg_len > client.max_message_length() {
                        Err(Error::Validation(format!(
                            "Message too long for Twitter: {} chars (max {})",
                            msg_len,
                            client.max_message_length()
                        )))
                    } else {
                        client.post(&account.access_token, post_request).await
                    }
                }
                crosspost_core::Platform::Facebook => {
                    let client = FacebookClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Instagram => {
                    let client = InstagramClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::LinkedIn => {
                    let client = LinkedInClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::YouTube => {
                    let client = YouTubeClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::TikTok => {
                    let client = TikTokClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Reddit => {
                    let client = RedditClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Twitch => {
                    let client_id = state
                        .config
                        .oauth
                        .twitch
                        .as_ref()
                        .map(|c| c.client_id.clone())
                        .unwrap_or_default();
                    let client = crosspost_platforms::twitch::TwitchClient::new(client_id);
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Slack => {
                    let client = SlackClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Telegram => {
                    let client = crosspost_platforms::telegram::TelegramClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Bluesky => {
                    let client = BlueskyClient::new();
                    let msg_len = client.calculate_message_length(&post_request.content);
                    if msg_len > client.max_message_length() {
                        Err(Error::Validation(format!(
                            "Message too long for Bluesky: {} chars (max {})",
                            msg_len,
                            client.max_message_length()
                        )))
                    } else {
                        client.post(&account.access_token, post_request).await
                    }
                }
                crosspost_core::Platform::Mastodon => {
                    let client = MastodonClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Discord => {
                    let client = DiscordClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::DiscordWebhook => {
                    let client = DiscordWebhookClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Devto => {
                    let client = DevtoClient::new();
                    client.post(&account.access_token, post_request).await
                }
                crosspost_core::Platform::Nostr => {
                    let client = NostrClient::new();
                    let msg_len = client.calculate_message_length(&post_request.content);
                    if msg_len > client.max_message_length() {
                        Err(Error::Validation(format!(
                            "Message too long for Nostr: {} chars (max {})",
                            msg_len,
                            client.max_message_length()
                        )))
                    } else {
                        client.post(&account.access_token, post_request).await
                    }
                }
            };

        let result = match platform_result {
            Ok(response) => PlatformPostResult {
                account_id: *account_id,
                platform: Some(account.platform),
                status: PostStatus::Success,
                platform_post_id: Some(response.platform_post_id.clone()),
                error_message: None,
            },
            Err(e) => {
                tracing::error!(
                    account_id = %account_id,
                    platform = %account.platform,
                    "Platform dispatch failed: {}",
                    e
                );
                PlatformPostResult {
                    account_id: *account_id,
                    platform: Some(account.platform),
                    status: PostStatus::Failed,
                    platform_post_id: None,
                    error_message: Some("Failed to post to platform".to_string()),
                }
            }
        };

        // Persist platform post record (only if we know the platform)
        if let Some(platform) = result.platform {
            let platform_post = PlatformPost {
                id: Uuid::new_v4(),
                post_id: post.id,
                account_id: *account_id,
                platform,
                platform_post_id: result.platform_post_id.clone(),
                status: result.status,
                error_message: result.error_message.clone(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            if let Err(e) = state.db.create_platform_post(platform_post).await {
                tracing::error!("Failed to persist platform post: {}", e);
            }
        }

        results.push(result);
    }

    Ok((
        StatusCode::CREATED,
        Json(CreatePostResponse {
            post_id: post.id,
            results,
        }),
    ))
}

/// List post history for the authenticated user
pub async fn list_posts(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<PaginationQuery>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);
    let posts = state
        .db
        .list_posts_by_user(claims.sub, limit, offset)
        .await?;
    Ok(Json(posts))
}

/// Schedule a post for future publishing
pub async fn schedule_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<SchedulePostRequest>,
) -> Result<impl IntoResponse, AppError> {
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| Error::Validation(e.to_string()))?;

    let scheduled_post = ScheduledPost {
        id: Uuid::new_v4(),
        user_id: claims.sub,
        tenant_id: claims.tenant_id,
        content: request.content,
        scheduled_for: request.scheduled_for,
        account_ids: request.account_ids,
        status: PostStatus::Scheduled,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let scheduled_post = state.db.create_scheduled_post(scheduled_post).await?;

    Ok((StatusCode::CREATED, Json(scheduled_post)))
}

/// List scheduled posts for the authenticated user
pub async fn list_scheduled_posts(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let posts = state.db.list_scheduled_posts_by_user(claims.sub).await?;
    Ok(Json(posts))
}

/// Update a scheduled post (only if still in scheduled status)
pub async fn update_scheduled_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(post_id): axum::extract::Path<Uuid>,
    Json(request): Json<UpdateScheduledPostRequest>,
) -> Result<impl IntoResponse, AppError> {
    request
        .validate()
        .map_err(|e: validator::ValidationErrors| Error::Validation(e.to_string()))?;

    let post = state
        .db
        .get_scheduled_post(post_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Scheduled post {} not found", post_id)))?;

    if post.user_id != claims.sub || post.tenant_id != claims.tenant_id {
        return Err(Error::Forbidden("You do not own this scheduled post".to_string()).into());
    }

    if post.status != PostStatus::Scheduled {
        return Err(
            Error::Validation("Can only update posts with scheduled status".to_string()).into(),
        );
    }

    state
        .db
        .update_scheduled_post(
            post_id,
            request.content,
            request.scheduled_for,
            request.account_ids,
        )
        .await?;

    let updated = state.db.get_scheduled_post(post_id).await?;
    Ok(Json(updated))
}

/// Cancel a scheduled post
pub async fn cancel_scheduled_post(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    axum::extract::Path(post_id): axum::extract::Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let post = state
        .db
        .get_scheduled_post(post_id)
        .await?
        .ok_or_else(|| Error::NotFound(format!("Scheduled post {} not found", post_id)))?;

    if post.user_id != claims.sub || post.tenant_id != claims.tenant_id {
        return Err(Error::Forbidden("You do not own this scheduled post".to_string()).into());
    }

    if post.status != PostStatus::Scheduled {
        return Err(
            Error::Validation("Can only cancel posts with scheduled status".to_string()).into(),
        );
    }

    state.db.delete_scheduled_post(post_id).await?;

    Ok(StatusCode::NO_CONTENT)
}
