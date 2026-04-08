use crate::server::core::{Error, Platform, Result};
use crate::server::db::SurrealDbClient;
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenUrl,
};
use std::sync::Arc;

pub struct OAuthHandler {
    _db: Arc<SurrealDbClient>,
}

impl OAuthHandler {
    pub fn new(db: Arc<SurrealDbClient>) -> Self {
        Self { _db: db }
    }

    /// Create OAuth client for a platform
    pub fn create_oauth_client(
        &self,
        platform: Platform,
        client_id: &str,
        client_secret: &str,
        redirect_uri: &str,
    ) -> Result<BasicClient> {
        let (auth_url, token_url) = self.get_platform_urls(platform)?;

        let client = BasicClient::new(
            ClientId::new(client_id.to_string()),
            Some(ClientSecret::new(client_secret.to_string())),
            AuthUrl::new(auth_url).map_err(|e| Error::OAuth(e.to_string()))?,
            Some(TokenUrl::new(token_url).map_err(|e| Error::OAuth(e.to_string()))?),
        )
        .set_redirect_uri(
            RedirectUrl::new(redirect_uri.to_string()).map_err(|e| Error::OAuth(e.to_string()))?,
        );

        Ok(client)
    }

    /// Check if a platform requires PKCE
    fn requires_pkce(platform: Platform) -> bool {
        matches!(platform, Platform::Twitter)
    }

    /// Get authorization URL for OAuth flow (with optional PKCE)
    pub fn get_authorization_url(
        &self,
        client: &BasicClient,
        platform: Platform,
    ) -> Result<(String, String, Option<PkceCodeVerifier>)> {
        let scopes = self.get_platform_scopes(platform);

        let mut auth_request = client.authorize_url(CsrfToken::new_random);

        for scope in scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.to_string()));
        }

        if Self::requires_pkce(platform) {
            let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
            auth_request = auth_request.set_pkce_challenge(pkce_challenge);
            let (auth_url, csrf_token) = auth_request.url();
            Ok((
                auth_url.to_string(),
                csrf_token.secret().clone(),
                Some(pkce_verifier),
            ))
        } else {
            let (auth_url, csrf_token) = auth_request.url();
            Ok((auth_url.to_string(), csrf_token.secret().clone(), None))
        }
    }

    /// Exchange authorization code for access token (with optional PKCE verifier)
    pub async fn exchange_code(
        &self,
        client: &BasicClient,
        code: String,
        pkce_verifier: Option<PkceCodeVerifier>,
    ) -> Result<
        oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>,
    > {
        let mut exchange = client.exchange_code(AuthorizationCode::new(code));

        if let Some(verifier) = pkce_verifier {
            exchange = exchange.set_pkce_verifier(verifier);
        }

        let token = exchange
            .request_async(async_http_client)
            .await
            .map_err(|e| Error::OAuth(e.to_string()))?;

        Ok(token)
    }

    /// Get platform-specific OAuth URLs
    fn get_platform_urls(&self, platform: Platform) -> Result<(String, String)> {
        let (auth_url, token_url) = match platform {
            Platform::Twitter => (
                "https://twitter.com/i/oauth2/authorize",
                "https://api.twitter.com/2/oauth2/token",
            ),
            Platform::Facebook => (
                "https://www.facebook.com/v18.0/dialog/oauth",
                "https://graph.facebook.com/v18.0/oauth/access_token",
            ),
            Platform::Instagram => (
                "https://api.instagram.com/oauth/authorize",
                "https://api.instagram.com/oauth/access_token",
            ),
            Platform::LinkedIn => (
                "https://www.linkedin.com/oauth/v2/authorization",
                "https://www.linkedin.com/oauth/v2/accessToken",
            ),
            Platform::YouTube => (
                "https://accounts.google.com/o/oauth2/v2/auth",
                "https://oauth2.googleapis.com/token",
            ),
            Platform::TikTok => (
                "https://www.tiktok.com/v2/auth/authorize",
                "https://open.tiktokapis.com/v2/oauth/token",
            ),
            Platform::Reddit => (
                "https://www.reddit.com/api/v1/authorize",
                "https://www.reddit.com/api/v1/access_token",
            ),
            Platform::Twitch => (
                "https://id.twitch.tv/oauth2/authorize",
                "https://id.twitch.tv/oauth2/token",
            ),
            Platform::Slack => (
                "https://slack.com/oauth/v2/authorize",
                "https://slack.com/api/oauth.v2.access",
            ),
            Platform::Telegram => {
                return Err(Error::OAuth(
                    "Telegram uses Bot API, not OAuth2".to_string(),
                ))
            }
            Platform::Bluesky => {
                return Err(Error::OAuth(
                    "Bluesky uses app password authentication, not OAuth2".to_string(),
                ))
            }
            Platform::Mastodon => (
                "https://mastodon.social/oauth/authorize",
                "https://mastodon.social/oauth/token",
            ),
            Platform::Discord | Platform::DiscordWebhook => (
                "https://discord.com/api/oauth2/authorize",
                "https://discord.com/api/oauth2/token",
            ),
            Platform::Devto => {
                return Err(Error::OAuth(
                    "Dev.to uses API key authentication, not OAuth2".to_string(),
                ))
            }
            Platform::Nostr => {
                return Err(Error::OAuth(
                    "Nostr uses private key authentication, not OAuth2".to_string(),
                ))
            }
        };

        Ok((auth_url.to_string(), token_url.to_string()))
    }

    /// Get platform-specific OAuth scopes
    fn get_platform_scopes(&self, platform: Platform) -> Vec<&str> {
        match platform {
            Platform::Twitter => vec!["tweet.read", "tweet.write", "users.read", "offline.access"],
            Platform::Facebook => vec!["pages_manage_posts", "pages_read_engagement"],
            Platform::Instagram => vec!["instagram_basic", "instagram_content_publish"],
            Platform::LinkedIn => vec!["w_member_social", "openid", "profile"],
            Platform::YouTube => vec![
                "https://www.googleapis.com/auth/youtube.upload",
                "https://www.googleapis.com/auth/youtube.force-ssl",
            ],
            Platform::TikTok => vec!["user.info.basic", "video.upload", "video.publish"],
            Platform::Reddit => vec!["submit", "identity"],
            Platform::Twitch => vec!["channel:manage:broadcast", "user:read:email"],
            Platform::Slack => vec!["chat:write", "files:write"],
            Platform::Telegram => vec![],
            Platform::Bluesky => vec![],
            Platform::Mastodon => vec!["read", "write"],
            Platform::Discord | Platform::DiscordWebhook => {
                vec!["bot", "applications.commands"]
            }
            Platform::Devto => vec![],
            Platform::Nostr => vec![],
        }
    }
}
