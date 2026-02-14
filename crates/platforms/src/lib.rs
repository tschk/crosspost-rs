pub mod platform_trait;
pub mod twitter;
pub mod facebook;
pub mod instagram;

pub use platform_trait::{Platform as PlatformClient, PostRequest, PostResponse};
