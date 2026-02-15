pub mod config;
pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

/// Prelude for convenient imports
///
/// ```rust
/// use crosspost_core::prelude::*;
/// ```
pub mod prelude {
    pub use crate::config::AppConfig;
    pub use crate::error::{Error, Result};
    pub use crate::types::{
        ConnectedAccount, CreatePostRequest, CreatePostResponse, DirectConnectRequest, ImageData,
        PaginationQuery, Platform, PlatformPost, PlatformPostResult, Post, PostStatus,
        SchedulePostRequest, ScheduledPost, Tenant, UpdateScheduledPostRequest, User,
    };
}
