use crosspost_core::{
    ConnectedAccount, Error, PlatformPost, Post, PostStatus, Result, ScheduledPost, Tenant, User,
};
use surrealdb::engine::any::{self, Any};
use surrealdb::Surreal;
use uuid::Uuid;

pub struct SurrealDbClient {
    db: Surreal<Any>,
}

impl SurrealDbClient {
    pub async fn new(url: &str) -> Result<Self> {
        let db = any::connect(url)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        db.use_ns("crosspost")
            .use_db("main")
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(Self { db })
    }

    pub fn get_db_handle(&self) -> Surreal<Any> {
        self.db.clone()
    }

    pub async fn init(&self) -> Result<()> {
        self.create_tables().await?;
        Ok(())
    }

    async fn create_tables(&self) -> Result<()> {
        let queries = [
            "DEFINE TABLE IF NOT EXISTS tenants SCHEMALESS",
            "DEFINE TABLE IF NOT EXISTS users SCHEMALESS",
            "DEFINE TABLE IF NOT EXISTS connected_accounts SCHEMALESS",
            "DEFINE TABLE IF NOT EXISTS posts SCHEMALESS",
            "DEFINE TABLE IF NOT EXISTS scheduled_posts SCHEMALESS",
            "DEFINE TABLE IF NOT EXISTS platform_posts SCHEMALESS",
            "DEFINE TABLE IF NOT EXISTS cache SCHEMALESS",
            // Indexes for common queries
            "DEFINE INDEX IF NOT EXISTS idx_users_tenant ON users FIELDS tenant_id",
            "DEFINE INDEX IF NOT EXISTS idx_accounts_user ON connected_accounts FIELDS user_id",
            "DEFINE INDEX IF NOT EXISTS idx_accounts_tenant ON connected_accounts FIELDS tenant_id",
            "DEFINE INDEX IF NOT EXISTS idx_posts_user ON posts FIELDS user_id",
            "DEFINE INDEX IF NOT EXISTS idx_posts_tenant ON posts FIELDS tenant_id",
            "DEFINE INDEX IF NOT EXISTS idx_scheduled_user ON scheduled_posts FIELDS user_id",
            "DEFINE INDEX IF NOT EXISTS idx_users_email ON users FIELDS email UNIQUE",
            "DEFINE INDEX IF NOT EXISTS idx_platform_posts_post ON platform_posts FIELDS post_id",
        ];

        for query in queries {
            self.db
                .query(query)
                .await
                .map_err(|e| Error::Database(e.to_string()))?;
        }

        Ok(())
    }

    // --- Tenants ---

    pub async fn create_tenant(&self, name: &str) -> Result<Tenant> {
        let tenant = Tenant {
            id: Uuid::new_v4(),
            name: name.to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let created: Option<Tenant> = self
            .db
            .create(("tenants", tenant.id.to_string()))
            .content(tenant)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        created.ok_or_else(|| Error::Database("Failed to create tenant".to_string()))
    }

    pub async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>> {
        let result: Option<Tenant> = self
            .db
            .select(("tenants", tenant_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result)
    }

    // --- Users ---

    pub async fn create_user_record(&self, user: User) -> Result<User> {
        let created: Option<User> = self
            .db
            .create(("users", user.id.to_string()))
            .content(user)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        created.ok_or_else(|| Error::Database("Failed to create user".to_string()))
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let result: Option<User> = self
            .db
            .select(("users", user_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let mut result = self
            .db
            .query("SELECT * FROM users WHERE email = $email LIMIT 1")
            .bind(("email", email.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let users: Vec<User> = result.take(0).map_err(|e| Error::Database(e.to_string()))?;

        Ok(users.into_iter().next())
    }

    // --- Connected Accounts ---

    pub async fn create_connected_account(
        &self,
        account: ConnectedAccount,
    ) -> Result<ConnectedAccount> {
        let created: Option<ConnectedAccount> = self
            .db
            .create(("connected_accounts", account.id.to_string()))
            .content(account)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        created.ok_or_else(|| Error::Database("Failed to create connected account".to_string()))
    }

    pub async fn get_connected_account(
        &self,
        account_id: Uuid,
    ) -> Result<Option<ConnectedAccount>> {
        let result: Option<ConnectedAccount> = self
            .db
            .select(("connected_accounts", account_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result)
    }

    pub async fn list_connected_accounts_by_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConnectedAccount>> {
        let mut result = self
            .db
            .query("SELECT * FROM connected_accounts WHERE user_id = $user_id")
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let accounts: Vec<ConnectedAccount> =
            result.take(0).map_err(|e| Error::Database(e.to_string()))?;

        Ok(accounts)
    }

    pub async fn update_connected_account_tokens(
        &self,
        account_id: Uuid,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        self.db
            .query("UPDATE connected_accounts SET access_token = $access_token, refresh_token = $refresh_token, token_expires_at = $expires_at, updated_at = $now WHERE id = $id")
            .bind(("id", account_id.to_string()))
            .bind(("access_token", access_token.to_string()))
            .bind(("refresh_token", refresh_token.map(|s| s.to_string())))
            .bind(("expires_at", expires_at))
            .bind(("now", chrono::Utc::now()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn delete_connected_account(&self, account_id: Uuid) -> Result<()> {
        // Delete related platform_posts and remove account from scheduled_posts
        self.db
            .query("DELETE FROM platform_posts WHERE account_id = $account_id")
            .bind(("account_id", account_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        // Remove this account_id from any scheduled posts' account_ids arrays
        self.db
            .query("UPDATE scheduled_posts SET account_ids = array::remove(account_ids, array::find_index(account_ids, $account_id)), updated_at = $now WHERE $account_id IN account_ids")
            .bind(("account_id", account_id.to_string()))
            .bind(("now", chrono::Utc::now()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let _: Option<ConnectedAccount> = self
            .db
            .delete(("connected_accounts", account_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    /// Get accounts with tokens expiring within the given number of minutes
    pub async fn get_accounts_with_expiring_tokens(
        &self,
        minutes: i64,
    ) -> Result<Vec<ConnectedAccount>> {
        let threshold = chrono::Utc::now() + chrono::Duration::minutes(minutes);
        let mut result = self
            .db
            .query("SELECT * FROM connected_accounts WHERE token_expires_at IS NOT NONE AND token_expires_at < $threshold AND refresh_token IS NOT NONE")
            .bind(("threshold", threshold))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let accounts: Vec<ConnectedAccount> =
            result.take(0).map_err(|e| Error::Database(e.to_string()))?;

        Ok(accounts)
    }

    // --- Posts ---

    pub async fn create_post(&self, post: Post) -> Result<Post> {
        let created: Option<Post> = self
            .db
            .create(("posts", post.id.to_string()))
            .content(post)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        created.ok_or_else(|| Error::Database("Failed to create post".to_string()))
    }

    pub async fn list_posts_by_user(
        &self,
        user_id: Uuid,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Post>> {
        let mut result = self
            .db
            .query("SELECT * FROM posts WHERE user_id = $user_id ORDER BY created_at DESC LIMIT $limit START $offset")
            .bind(("user_id", user_id.to_string()))
            .bind(("limit", limit))
            .bind(("offset", offset))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let posts: Vec<Post> = result.take(0).map_err(|e| Error::Database(e.to_string()))?;

        Ok(posts)
    }

    // --- Scheduled Posts ---

    pub async fn create_scheduled_post(&self, post: ScheduledPost) -> Result<ScheduledPost> {
        let created: Option<ScheduledPost> = self
            .db
            .create(("scheduled_posts", post.id.to_string()))
            .content(post)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        created.ok_or_else(|| Error::Database("Failed to create scheduled post".to_string()))
    }

    pub async fn list_scheduled_posts_by_user(&self, user_id: Uuid) -> Result<Vec<ScheduledPost>> {
        let mut result = self
            .db
            .query("SELECT * FROM scheduled_posts WHERE user_id = $user_id AND status = 'scheduled' ORDER BY scheduled_for ASC")
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let posts: Vec<ScheduledPost> =
            result.take(0).map_err(|e| Error::Database(e.to_string()))?;

        Ok(posts)
    }

    pub async fn get_scheduled_post(&self, id: Uuid) -> Result<Option<ScheduledPost>> {
        let result: Option<ScheduledPost> = self
            .db
            .select(("scheduled_posts", id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(result)
    }

    pub async fn delete_scheduled_post(&self, id: Uuid) -> Result<()> {
        let _: Option<ScheduledPost> = self
            .db
            .delete(("scheduled_posts", id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn update_scheduled_post(
        &self,
        id: Uuid,
        content: Option<String>,
        scheduled_for: Option<chrono::DateTime<chrono::Utc>>,
        account_ids: Option<Vec<Uuid>>,
    ) -> Result<()> {
        let mut updates = vec!["updated_at = $now"];
        if content.is_some() {
            updates.push("content = $content");
        }
        if scheduled_for.is_some() {
            updates.push("scheduled_for = $scheduled_for");
        }
        if account_ids.is_some() {
            updates.push("account_ids = $account_ids");
        }

        let query = format!(
            "UPDATE scheduled_posts SET {} WHERE id = $id AND status = 'scheduled'",
            updates.join(", ")
        );

        self.db
            .query(&query)
            .bind(("id", id.to_string()))
            .bind(("now", chrono::Utc::now()))
            .bind(("content", content))
            .bind(("scheduled_for", scheduled_for))
            .bind((
                "account_ids",
                account_ids.map(|ids| ids.iter().map(|id| id.to_string()).collect::<Vec<_>>()),
            ))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    // --- Platform Posts ---

    pub async fn create_platform_post(&self, post: PlatformPost) -> Result<PlatformPost> {
        let created: Option<PlatformPost> = self
            .db
            .create(("platform_posts", post.id.to_string()))
            .content(post)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        created.ok_or_else(|| Error::Database("Failed to create platform post".to_string()))
    }

    pub async fn list_platform_posts_by_post(&self, post_id: Uuid) -> Result<Vec<PlatformPost>> {
        let mut result = self
            .db
            .query("SELECT * FROM platform_posts WHERE post_id = $post_id ORDER BY created_at DESC")
            .bind(("post_id", post_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let posts: Vec<PlatformPost> =
            result.take(0).map_err(|e| Error::Database(e.to_string()))?;

        Ok(posts)
    }

    // --- Scheduled Post Execution ---

    pub async fn get_due_scheduled_posts(&self) -> Result<Vec<ScheduledPost>> {
        let mut result = self
            .db
            .query("SELECT * FROM scheduled_posts WHERE status = 'scheduled' AND scheduled_for <= time::now()")
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let posts: Vec<ScheduledPost> =
            result.take(0).map_err(|e| Error::Database(e.to_string()))?;

        Ok(posts)
    }

    pub async fn update_scheduled_post_status(&self, id: Uuid, status: PostStatus) -> Result<()> {
        self.db
            .query("UPDATE scheduled_posts SET status = $status, updated_at = $now WHERE id = $id")
            .bind(("id", id.to_string()))
            .bind((
                "status",
                serde_json::to_value(status)
                    .unwrap_or_default()
                    .as_str()
                    .unwrap_or("failed")
                    .to_string(),
            ))
            .bind(("now", chrono::Utc::now()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl crate::Database for SurrealDbClient {
    async fn init(&self) -> Result<()> {
        self.init().await
    }

    async fn health_check(&self) -> Result<()> {
        self.db
            .health()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }
}
