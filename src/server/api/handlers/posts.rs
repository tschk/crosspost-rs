use crate::server::api::{middleware::AppError, state::AppState};
use crate::server::auth::Claims;
use crate::server::core::{
    ConnectedAccount, CreatePostRequest, CreatePostResponse, Error, PaginationQuery, Platform,
    PlatformPost, PlatformPostResult, Post, PostStatus, SchedulePostRequest, ScheduledPost,
    UpdateScheduledPostRequest,
};
use crate::{
    BlueskyCredentials, BlueskyStrategy, DevtoCredentials, DevtoStrategy, DiscordCredentials,
    DiscordStrategy, DiscordWebhookCredentials, DiscordWebhookStrategy, FacebookCredentials,
    FacebookStrategy, ImageEmbed, InstagramCredentials, InstagramStrategy, LinkedInCredentials,
    LinkedInStrategy, MastodonCredentials, MastodonStrategy, NostrCredentials, NostrStrategy,
    PostOptions, RedditCredentials, RedditStrategy, SlackCredentials, SlackStrategy, Strategy,
    TelegramCredentials, TelegramStrategy, TikTokCredentials, TikTokStrategy, TwitchCredentials,
    TwitchStrategy, TwitterCredentials, TwitterStrategy, YouTubeCredentials, YouTubeStrategy,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

/// Create a Strategy from a ConnectedAccount (maps pipe-delimited tokens to typed credentials)
pub fn create_strategy_for_account(
    account: &ConnectedAccount,
    state: &AppState,
) -> Result<Box<dyn Strategy>, Error> {
    match account.platform {
        Platform::Twitter => {
            let strategy = TwitterStrategy::new(TwitterCredentials {
                access_token: account.access_token.clone(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Facebook => {
            let strategy = FacebookStrategy::new(FacebookCredentials {
                access_token: account.access_token.clone(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Instagram => {
            let strategy = InstagramStrategy::new(InstagramCredentials {
                access_token: account.access_token.clone(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::LinkedIn => {
            let strategy = LinkedInStrategy::new(LinkedInCredentials {
                access_token: account.access_token.clone(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::YouTube => {
            let strategy = YouTubeStrategy::new(YouTubeCredentials {
                access_token: account.access_token.clone(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::TikTok => {
            let strategy = TikTokStrategy::new(TikTokCredentials {
                access_token: account.access_token.clone(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Reddit => {
            let strategy = RedditStrategy::new(RedditCredentials {
                access_token: account.access_token.clone(),
                subreddit: None,
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Twitch => {
            let client_id = state
                .config
                .oauth
                .twitch
                .as_ref()
                .map(|c| c.client_id.clone())
                .unwrap_or_default();
            let strategy = TwitchStrategy::new(TwitchCredentials {
                access_token: account.access_token.clone(),
                client_id,
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Slack => {
            // Slack stores bot_token, optionally bot_token|channel
            let parts: Vec<&str> = account.access_token.splitn(2, '|').collect();
            let strategy = SlackStrategy::new(SlackCredentials {
                bot_token: parts[0].to_string(),
                channel: parts.get(1).map(|s| s.to_string()),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Telegram => {
            // Telegram stores bot_token|chat_id
            let parts: Vec<&str> = account.access_token.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err(Error::Platform(
                    "Invalid Telegram token format (expected bot_token|chat_id)".to_string(),
                ));
            }
            let strategy = TelegramStrategy::new(TelegramCredentials {
                bot_token: parts[0].to_string(),
                chat_id: parts[1].to_string(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Bluesky => {
            // Bluesky stores identifier|password
            let parts: Vec<&str> = account.access_token.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err(Error::Platform(
                    "Invalid Bluesky token format (expected identifier|password)".to_string(),
                ));
            }
            let strategy = BlueskyStrategy::new(BlueskyCredentials {
                identifier: parts[0].to_string(),
                password: parts[1].to_string(),
                host: None,
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Mastodon => {
            // Mastodon stores access_token|host
            let parts: Vec<&str> = account.access_token.splitn(2, '|').collect();
            let host = parts
                .get(1)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "mastodon.social".to_string());
            let strategy = MastodonStrategy::new(MastodonCredentials {
                access_token: parts[0].to_string(),
                host,
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Discord => {
            // Discord stores bot_token|channel_id
            let parts: Vec<&str> = account.access_token.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err(Error::Platform(
                    "Invalid Discord token format (expected bot_token|channel_id)".to_string(),
                ));
            }
            let strategy = DiscordStrategy::new(DiscordCredentials {
                bot_token: parts[0].to_string(),
                channel_id: parts[1].to_string(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::DiscordWebhook => {
            let strategy = DiscordWebhookStrategy::new(DiscordWebhookCredentials {
                webhook_url: account.access_token.clone(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Devto => {
            let strategy = DevtoStrategy::new(DevtoCredentials {
                api_key: account.access_token.clone(),
            })
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
        Platform::Nostr => {
            // Nostr stores private_key|relay1,relay2
            let parts: Vec<&str> = account.access_token.splitn(2, '|').collect();
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
            .map_err(|e| Error::Platform(e.to_string()))?;
            Ok(Box::new(strategy))
        }
    }
}

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
            let oauth_config = super::auth::get_platform_oauth_config(&state, account.platform);
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

        // Convert images from CreatePostRequest format to ImageEmbed
        let images: Vec<ImageEmbed> = if let Some(ref img_data_list) = request.images {
            let mut embeds = Vec::new();
            for img_data in img_data_list {
                if let Some(ref data_b64) = img_data.data {
                    let bytes = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        data_b64,
                    )
                    .map_err(|e| Error::Validation(format!("Invalid base64 image data: {}", e)))?;
                    let mime = crate::util::images::detect_mime_type(&bytes)
                        .unwrap_or_else(|_| "image/jpeg".to_string());
                    let compressed =
                        crate::util::images::compress_image(&bytes, &mime).unwrap_or(bytes);
                    embeds.push(ImageEmbed {
                        data: compressed,
                        alt: img_data.alt.clone(),
                        mime_type: Some(mime),
                        image_url: None,
                    });
                }
            }
            embeds
        } else {
            Vec::new()
        };

        let options = PostOptions {
            images: images.clone(),
        };

        // Create strategy and post
        let strategy = match create_strategy_for_account(&account, &state) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    account_id = %account_id,
                    platform = %account.platform,
                    "Failed to create strategy: {}",
                    e
                );
                results.push(PlatformPostResult {
                    account_id: *account_id,
                    platform: Some(account.platform),
                    status: PostStatus::Failed,
                    platform_post_id: None,
                    error_message: Some("Failed to create platform strategy".to_string()),
                });
                continue;
            }
        };

        // Message length validation
        let msg_len = strategy.calculate_message_length(&request.content);
        let max_len = strategy.max_message_length();
        if msg_len > max_len {
            results.push(PlatformPostResult {
                account_id: *account_id,
                platform: Some(account.platform),
                status: PostStatus::Failed,
                platform_post_id: None,
                error_message: Some(format!(
                    "Message too long for {}: {} chars (max {})",
                    strategy.name(),
                    msg_len,
                    max_len
                )),
            });
            continue;
        }

        let platform_result = strategy.post(&request.content, Some(&options)).await;

        let result = match platform_result {
            Ok(response) => PlatformPostResult {
                account_id: *account_id,
                platform: Some(account.platform),
                status: PostStatus::Success,
                platform_post_id: Some(response.id.clone()),
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

        // Persist platform post record
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
