use crosspost_core::{Error, Result};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

pub struct RocksDbClient {
    db: Surreal<Any>,
}

impl RocksDbClient {
    pub async fn new(db: Surreal<Any>) -> Result<Self> {
        Ok(Self { db })
    }

    /// Store a token in the cache
    pub async fn store_token(&self, key: &str, value: &str) -> Result<()> {
        let cache_key = format!("cache:{}", key);
        let _: Option<String> = self
            .db
            .set(&cache_key, value)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Retrieve a token from the cache
    pub async fn get_token(&self, key: &str) -> Result<Option<String>> {
        let cache_key = format!("cache:{}", key);
        let result: Option<String> = self
            .db
            .select(&cache_key)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(result)
    }

    /// Delete a token from the cache
    pub async fn delete_token(&self, key: &str) -> Result<()> {
        let cache_key = format!("cache:{}", key);
        let _: Option<String> = self
            .db
            .delete(&cache_key)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Increment a rate limit counter
    pub async fn increment_rate_limit(&self, key: &str) -> Result<u64> {
        let current = self.get_rate_limit_count(key).await?;
        let new_count = current + 1;
        self.store_token(key, &new_count.to_string()).await?;
        Ok(new_count)
    }

    /// Get current rate limit count
    pub async fn get_rate_limit_count(&self, key: &str) -> Result<u64> {
        match self.get_token(key).await? {
            Some(value) => {
                let count = value
                    .parse::<u64>()
                    .map_err(|e| Error::Database(e.to_string()))?;
                Ok(count)
            }
            None => Ok(0),
        }
    }

    /// Reset rate limit counter
    pub async fn reset_rate_limit(&self, key: &str) -> Result<()> {
        self.delete_token(key).await
    }
}

#[async_trait::async_trait]
impl crate::Database for RocksDbClient {
    async fn init(&self) -> Result<()> {
        // No initialization needed
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        // Simple health check
        self.db
            .health()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }
}
