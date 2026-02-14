pub mod surrealdb_client;
pub mod rocksdb_client;

pub use surrealdb_client::SurrealDbClient;
pub use rocksdb_client::RocksDbClient;

use crosspost_core::Result;

/// Database trait for abstracting database operations
#[async_trait::async_trait]
pub trait Database: Send + Sync {
    async fn init(&self) -> Result<()>;
    async fn health_check(&self) -> Result<()>;
}
