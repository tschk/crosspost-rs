use super::state::AppState;
use crate::server::core::{Platform, PostStatus};
use std::sync::Arc;
use tokio::time::{interval, Duration};

/// Background scheduler that executes due scheduled posts every 30 seconds.
/// Text-only: no media persistence.
pub async fn run_scheduler(state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(30));

    loop {
        ticker.tick().await;

        match state.db.get_due_scheduled_posts().await {
            Ok(posts) => {
                for scheduled_post in posts {
                    tracing::info!(
                        "Executing scheduled post {} for user {}",
                        scheduled_post.id,
                        scheduled_post.user_id
                    );

                    let mut all_success = true;

                    for account_id in &scheduled_post.account_ids {
                        let account = match state.db.get_connected_account(*account_id).await {
                            Ok(Some(acc)) => acc,
                            _ => {
                                tracing::warn!(
                                    "Scheduled post {}: account {} not found",
                                    scheduled_post.id,
                                    account_id
                                );
                                all_success = false;
                                continue;
                            }
                        };

                        // Verify account belongs to the scheduled post's owner
                        if account.user_id != scheduled_post.user_id
                            || account.tenant_id != scheduled_post.tenant_id
                        {
                            tracing::warn!(
                                "Scheduled post {}: account {} not owned by user {}",
                                scheduled_post.id,
                                account_id,
                                scheduled_post.user_id
                            );
                            all_success = false;
                            continue;
                        }

                        let strategy = match super::handlers::posts::create_strategy_for_account(
                            &account, &state,
                        ) {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::error!(
                                        "Scheduled post {}: failed to create strategy for account {}: {}",
                                        scheduled_post.id,
                                        account_id,
                                        e
                                    );
                                all_success = false;
                                continue;
                            }
                        };

                        match strategy.post(&scheduled_post.content, None).await {
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(
                                    "Scheduled post {} failed for account {}: {}",
                                    scheduled_post.id,
                                    account_id,
                                    e
                                );
                                all_success = false;
                            }
                        }
                    }

                    let new_status = if all_success {
                        PostStatus::Success
                    } else {
                        PostStatus::Failed
                    };

                    if let Err(e) = state
                        .db
                        .update_scheduled_post_status(scheduled_post.id, new_status)
                        .await
                    {
                        tracing::error!(
                            "Failed to update scheduled post {} status: {}",
                            scheduled_post.id,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to query due scheduled posts: {}", e);
            }
        }
    }
}

/// Background task that proactively refreshes tokens expiring within 15 minutes.
/// Runs every 5 minutes.
pub async fn run_token_refresh(state: Arc<AppState>) {
    let mut ticker = interval(Duration::from_secs(300)); // every 5 minutes

    loop {
        ticker.tick().await;

        match state.db.get_accounts_with_expiring_tokens(15).await {
            Ok(accounts) => {
                for mut account in accounts {
                    // Skip non-OAuth platforms
                    if matches!(
                        account.platform,
                        Platform::Bluesky
                            | Platform::Telegram
                            | Platform::Nostr
                            | Platform::Devto
                            | Platform::DiscordWebhook
                    ) {
                        continue;
                    }

                    let oauth_config =
                        super::handlers::auth::get_platform_oauth_config(&state, account.platform);
                    if let Ok(oauth_cfg) = oauth_config {
                        if let Ok(oauth_client) = state.oauth_handler.create_oauth_client(
                            account.platform,
                            &oauth_cfg.client_id,
                            &oauth_cfg.client_secret,
                            &oauth_cfg.redirect_uri,
                        ) {
                            match state
                                .token_manager
                                .refresh_token_if_needed(&mut account, &oauth_client)
                                .await
                            {
                                Ok(()) => {
                                    tracing::debug!(
                                        "Proactively refreshed token for account {}",
                                        account.id
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to proactively refresh token for account {}: {}",
                                        account.id,
                                        e
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to query expiring tokens: {}", e);
            }
        }
    }
}
