pub mod cache_client;
pub mod surrealdb_client;

pub use cache_client::CacheClient;
pub use surrealdb_client::SurrealDbClient;

use crosspost_core::Result;

/// Database trait for abstracting database operations
#[async_trait::async_trait]
pub trait Database: Send + Sync {
    async fn init(&self) -> Result<()>;
    async fn health_check(&self) -> Result<()>;
}

/// Prelude for convenient imports
///
/// ```rust
/// use crosspost_db::prelude::*;
/// ```
pub mod prelude {
    pub use crate::{CacheClient, Database, SurrealDbClient};
}
