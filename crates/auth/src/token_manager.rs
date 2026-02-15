use chrono::{Duration, Utc};
use crosspost_core::{ConnectedAccount, Error, Result};
use crosspost_db::SurrealDbClient;
use oauth2::basic::BasicClient;
use oauth2::TokenResponse;
use std::sync::Arc;
use uuid::Uuid;

pub struct TokenManager {
    db: Arc<SurrealDbClient>,
    _oauth_handler: Arc<crate::OAuthHandler>,
}

impl TokenManager {
    pub fn new(db: Arc<SurrealDbClient>, oauth_handler: Arc<crate::OAuthHandler>) -> Self {
        Self {
            db,
            _oauth_handler: oauth_handler,
        }
    }

    /// Check if token is expired or will expire soon (within 5 minutes)
    pub fn is_token_expired(&self, account: &ConnectedAccount) -> bool {
        if let Some(expires_at) = account.token_expires_at {
            let now = Utc::now();
            let buffer = Duration::minutes(5);
            expires_at - buffer < now
        } else {
            false
        }
    }

    /// Refresh access token if needed
    pub async fn refresh_token_if_needed(
        &self,
        account: &mut ConnectedAccount,
        oauth_client: &BasicClient,
    ) -> Result<()> {
        if !self.is_token_expired(account) {
            return Ok(());
        }

        if let Some(refresh_token) = &account.refresh_token {
            let refresh_token = oauth2::RefreshToken::new(refresh_token.clone());

            let token_result = oauth_client
                .exchange_refresh_token(&refresh_token)
                .request_async(oauth2::reqwest::async_http_client)
                .await
                .map_err(|e| Error::OAuth(format!("Failed to refresh token: {}", e)))?;

            let new_access_token = token_result.access_token().secret().clone();
            let new_refresh_token = token_result.refresh_token().map(|t| t.secret().clone());
            let expires_at = token_result
                .expires_in()
                .map(|duration| Utc::now() + Duration::seconds(duration.as_secs() as i64));

            // Update in database
            self.db
                .update_connected_account_tokens(
                    account.id,
                    &new_access_token,
                    new_refresh_token.as_deref(),
                    expires_at,
                )
                .await?;

            // Update in-memory account
            account.access_token = new_access_token;
            account.refresh_token = new_refresh_token;
            account.token_expires_at = expires_at;
            account.updated_at = Utc::now();

            tracing::info!(
                "Refreshed token for account {} on platform {}",
                account.id,
                account.platform
            );

            Ok(())
        } else {
            Err(Error::OAuth(
                "No refresh token available for account".to_string(),
            ))
        }
    }

    /// Get account with fresh token (refresh if needed)
    pub async fn get_account_with_fresh_token(
        &self,
        account_id: Uuid,
        oauth_client: &BasicClient,
    ) -> Result<ConnectedAccount> {
        let mut account = self
            .db
            .get_connected_account(account_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Account {} not found", account_id)))?;

        self.refresh_token_if_needed(&mut account, oauth_client)
            .await?;

        Ok(account)
    }
}
