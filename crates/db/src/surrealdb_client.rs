use crosspost_core::{ConnectedAccount, Error, Post, Result, ScheduledPost, Tenant, User};
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
        let _: Option<ConnectedAccount> = self
            .db
            .delete(("connected_accounts", account_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
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

    pub async fn list_posts_by_user(&self, user_id: Uuid, limit: usize) -> Result<Vec<Post>> {
        let mut result = self
            .db
            .query("SELECT * FROM posts WHERE user_id = $user_id ORDER BY created_at DESC LIMIT $limit")
            .bind(("user_id", user_id.to_string()))
            .bind(("limit", limit))
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
