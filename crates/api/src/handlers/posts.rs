use crate::{middleware::AppError, state::AppState};
use axum::{extract::State, response::IntoResponse, Extension, Json};
use crosspost_auth::Claims;
use crosspost_core::{
    CreatePostRequest, CreatePostResponse, Error, PlatformPostResult, Post, PostStatus,
    SchedulePostRequest, ScheduledPost,
};
use crosspost_platforms::{
    facebook::FacebookClient, instagram::InstagramClient, linkedin::LinkedInClient,
    reddit::RedditClient, slack::SlackClient, tiktok::TikTokClient, twitter::TwitterClient,
    youtube::YouTubeClient, PlatformClient, PostRequest,
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
                        platform: crosspost_core::Platform::Twitter,
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
                    platform: crosspost_core::Platform::Twitter,
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

        let post_request = PostRequest {
            content: request.content.clone(),
            media_urls: request.media_urls.clone(),
        };

        let platform_result: crosspost_core::Result<crosspost_platforms::PostResponse> =
            match account.platform {
                crosspost_core::Platform::Twitter => {
                    let client = TwitterClient::new();
                    client.post(&account.access_token, post_request).await
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
            };

        let result = match platform_result {
            Ok(response) => PlatformPostResult {
                account_id: *account_id,
                platform: account.platform,
                status: PostStatus::Success,
                platform_post_id: Some(response.platform_post_id),
                error_message: None,
            },
            Err(e) => PlatformPostResult {
                account_id: *account_id,
                platform: account.platform,
                status: PostStatus::Failed,
                platform_post_id: None,
                error_message: Some(e.to_string()),
            },
        };

        results.push(result);
    }

    Ok(Json(CreatePostResponse {
        post_id: post.id,
        results,
    }))
}

/// List post history for the authenticated user
pub async fn list_posts(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let posts = state.db.list_posts_by_user(claims.sub, 50).await?;
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

    Ok(Json(scheduled_post))
}
