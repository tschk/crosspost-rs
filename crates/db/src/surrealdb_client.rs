use crosspost_core::{
    ConnectedAccount, Error, Platform, Post, Result, ScheduledPost, Tenant, User,
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
        // Initialize database schema
        self.create_tables().await?;
        Ok(())
    }

    async fn create_tables(&self) -> Result<()> {
        // Create tables/collections
        let _ = self
            .db
            .query("DEFINE TABLE IF NOT EXISTS tenants SCHEMAFULL")
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let _ = self
            .db
            .query("DEFINE TABLE IF NOT EXISTS users SCHEMAFULL")
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let _ = self
            .db
            .query("DEFINE TABLE IF NOT EXISTS connected_accounts SCHEMAFULL")
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let _ = self
            .db
            .query("DEFINE TABLE IF NOT EXISTS posts SCHEMAFULL")
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let _ = self
            .db
            .query("DEFINE TABLE IF NOT EXISTS scheduled_posts SCHEMAFULL")
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(())
    }

    pub async fn create_tenant(&self, name: &str) -> Result<Tenant> {
        let tenant = Tenant {
            id: Uuid::new_v4(),
            name: name.to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let _: Vec<Tenant> = self
            .db
            .create("tenants")
            .content(&tenant)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(tenant)
    }

    pub async fn get_tenant(&self, tenant_id: Uuid) -> Result<Option<Tenant>> {
        let result: Option<Tenant> = self
            .db
            .select(("tenants", tenant_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result)
    }

    pub async fn create_user(&self, tenant_id: Uuid, email: &str, name: &str) -> Result<User> {
        let user = User {
            id: Uuid::new_v4(),
            tenant_id,
            email: email.to_string(),
            name: name.to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let _: Vec<User> = self
            .db
            .create("users")
            .content(&user)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(user)
    }

    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<User>> {
        let result: Option<User> = self
            .db
            .select(("users", user_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result)
    }

    pub async fn create_connected_account(&self, account: ConnectedAccount) -> Result<ConnectedAccount> {
        let _: Vec<ConnectedAccount> = self
            .db
            .create("connected_accounts")
            .content(&account)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(account)
    }

    pub async fn get_connected_account(&self, account_id: Uuid) -> Result<Option<ConnectedAccount>> {
        let result: Option<ConnectedAccount> = self
            .db
            .select(("connected_accounts", account_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(result)
    }

    pub async fn list_connected_accounts_by_user(&self, user_id: Uuid) -> Result<Vec<ConnectedAccount>> {
        let query = "SELECT * FROM connected_accounts WHERE user_id = $user_id";
        let mut result = self
            .db
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let accounts: Vec<ConnectedAccount> = result
            .take(0)
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(accounts)
    }

    pub async fn update_connected_account_tokens(
        &self,
        account_id: Uuid,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<()> {
        let query = "UPDATE connected_accounts SET access_token = $access_token, refresh_token = $refresh_token, token_expires_at = $expires_at, updated_at = $now WHERE id = $id";
        
        self.db
            .query(query)
            .bind(("id", account_id.to_string()))
            .bind(("access_token", access_token))
            .bind(("refresh_token", refresh_token))
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

    pub async fn create_post(&self, post: Post) -> Result<Post> {
        let _: Vec<Post> = self
            .db
            .create("posts")
            .content(&post)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(post)
    }

    pub async fn list_posts_by_user(&self, user_id: Uuid, limit: usize) -> Result<Vec<Post>> {
        let query = "SELECT * FROM posts WHERE user_id = $user_id ORDER BY created_at DESC LIMIT $limit";
        let mut result = self
            .db
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .bind(("limit", limit))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let posts: Vec<Post> = result
            .take(0)
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(posts)
    }

    pub async fn create_scheduled_post(&self, post: ScheduledPost) -> Result<ScheduledPost> {
        let _: Vec<ScheduledPost> = self
            .db
            .create("scheduled_posts")
            .content(&post)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        Ok(post)
    }

    pub async fn list_scheduled_posts_by_user(&self, user_id: Uuid) -> Result<Vec<ScheduledPost>> {
        let query = "SELECT * FROM scheduled_posts WHERE user_id = $user_id AND status = 'scheduled' ORDER BY scheduled_for ASC";
        let mut result = self
            .db
            .query(query)
            .bind(("user_id", user_id.to_string()))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;

        let posts: Vec<ScheduledPost> = result
            .take(0)
            .map_err(|e| Error::Database(e.to_string()))?;

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
