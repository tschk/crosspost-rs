use crate::server::core::{Error, Result};
use serde::{Deserialize, Serialize};
use surrealdb::engine::any::Any;
use surrealdb::Surreal;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub struct CacheClient {
    db: Surreal<Any>,
}

impl CacheClient {
    pub async fn new(db: Surreal<Any>) -> Result<Self> {
        Ok(Self { db })
    }

    /// Store a value in the cache
    pub async fn store(&self, key: &str, value: &str) -> Result<()> {
        let entry = CacheEntry {
            value: value.to_string(),
            expires_at: None,
        };
        let _: Option<CacheEntry> = self
            .db
            .upsert(("cache", key))
            .content(entry)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Store a value in the cache with a TTL in seconds
    pub async fn store_with_ttl(&self, key: &str, value: &str, ttl_secs: i64) -> Result<()> {
        let entry = CacheEntry {
            value: value.to_string(),
            expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(ttl_secs)),
        };
        let _: Option<CacheEntry> = self
            .db
            .upsert(("cache", key))
            .content(entry)
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Retrieve a value from the cache, filtering out expired entries
    pub async fn get(&self, key: &str) -> Result<Option<String>> {
        let result: Option<CacheEntry> = self
            .db
            .select(("cache", key))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        match result {
            Some(entry) => {
                if let Some(expires_at) = entry.expires_at {
                    if chrono::Utc::now() > expires_at {
                        // Expired - clean up and return None
                        self.delete(key).await.ok();
                        return Ok(None);
                    }
                }
                Ok(Some(entry.value))
            }
            None => Ok(None),
        }
    }

    /// Delete a value from the cache
    pub async fn delete(&self, key: &str) -> Result<()> {
        let _: Option<CacheEntry> = self
            .db
            .delete(("cache", key))
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }

    /// Store a token (alias for store)
    pub async fn store_token(&self, key: &str, value: &str) -> Result<()> {
        self.store(key, value).await
    }

    /// Get a token (alias for get)
    pub async fn get_token(&self, key: &str) -> Result<Option<String>> {
        self.get(key).await
    }

    /// Delete a token (alias for delete)
    pub async fn delete_token(&self, key: &str) -> Result<()> {
        self.delete(key).await
    }

    /// Increment a rate limit counter
    pub async fn increment_rate_limit(&self, key: &str) -> Result<u64> {
        let current = self.get_rate_limit_count(key).await?;
        let new_count = current + 1;
        self.store(key, &new_count.to_string()).await?;
        Ok(new_count)
    }

    /// Get current rate limit count
    pub async fn get_rate_limit_count(&self, key: &str) -> Result<u64> {
        match self.get(key).await? {
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
        self.delete(key).await
    }
}

#[async_trait::async_trait]
impl super::Database for CacheClient {
    async fn init(&self) -> Result<()> {
        Ok(())
    }

    async fn health_check(&self) -> Result<()> {
        self.db
            .health()
            .await
            .map_err(|e| Error::Database(e.to_string()))?;
        Ok(())
    }
}
