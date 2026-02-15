pub mod jwt;
pub mod oauth;
pub mod password;
pub mod token_manager;

pub use jwt::{Claims, JwtManager};
pub use oauth::OAuthHandler;
pub use password::{hash_password, verify_password};
pub use token_manager::TokenManager;

/// Prelude for convenient imports
///
/// ```rust
/// use crosspost_auth::prelude::*;
/// ```
pub mod prelude {
    pub use crate::jwt::{Claims, JwtManager};
    pub use crate::oauth::OAuthHandler;
    pub use crate::password::{hash_password, verify_password};
    pub use crate::token_manager::TokenManager;
}
