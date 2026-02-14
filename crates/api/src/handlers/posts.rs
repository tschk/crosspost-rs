use crate::{middleware::AppError, state::AppState};
use axum::{extract::State, response::IntoResponse, Json};
use crosspost_core::{
    CreatePostRequest, CreatePostResponse, Error, PlatformPostResult, Post, PostStatus,
    SchedulePostRequest, ScheduledPost,
};
use crosspost_platforms::{
    facebook::FacebookClient, instagram::InstagramClient, twitter::TwitterClient, Platform as PlatformClient, PostRequest,
};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// Create and post content to multiple platforms
pub async fn create_post(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreatePostRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate request
    request.validate()
        .map_err(|e| Error::Validation(e.to_string()))?;

    // TODO: Get user_id and tenant_id from authenticated session
    let user_id = Uuid::new_v4(); // Placeholder
    let tenant_id = Uuid::new_v4(); // Placeholder

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
        
        let account = match account {
            Some(acc) => acc,
            None => {
                results.push(PlatformPostResult {
                    account_id: *account_id,
                    platform: crosspost_core::Platform::Twitter, // Placeholder
                    status: PostStatus::Failed,
                    platform_post_id: None,
                    error_message: Some("Account not found".to_string()),
                });
                continue;
            }
        };

        // Get platform client based on account platform
        let platform_result = match account.platform {
            crosspost_core::Platform::Twitter => {
                let client = TwitterClient::new();
                let post_request = PostRequest {
                    content: request.content.clone(),
                    media_urls: request.media_urls.clone(),
                };
                client.post(&account.access_token, post_request).await
            }
            crosspost_core::Platform::Facebook => {
                let client = FacebookClient::new();
                let post_request = PostRequest {
                    content: request.content.clone(),
                    media_urls: request.media_urls.clone(),
                };
                client.post(&account.access_token, post_request).await
            }
            crosspost_core::Platform::Instagram => {
                let client = InstagramClient::new();
                let post_request = PostRequest {
                    content: request.content.clone(),
                    media_urls: request.media_urls.clone(),
                };
                client.post(&account.access_token, post_request).await
            }
            _ => {
                results.push(PlatformPostResult {
                    account_id: *account_id,
                    platform: account.platform,
                    status: PostStatus::Failed,
                    platform_post_id: None,
                    error_message: Some("Platform not yet implemented".to_string()),
                });
                continue;
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

/// List post history for a user
pub async fn list_posts(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Get user_id from authenticated session
    let user_id = Uuid::new_v4(); // Placeholder

    let posts = state.db.list_posts_by_user(user_id, 50).await?;

    Ok(Json(posts))
}

/// Schedule a post for future publishing
pub async fn schedule_post(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SchedulePostRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate request
    request.validate()
        .map_err(|e| Error::Validation(e.to_string()))?;

    // TODO: Get user_id and tenant_id from authenticated session
    let user_id = Uuid::new_v4(); // Placeholder
    let tenant_id = Uuid::new_v4(); // Placeholder

    // Create scheduled post
    let scheduled_post = ScheduledPost {
        id: Uuid::new_v4(),
        user_id,
        tenant_id,
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
