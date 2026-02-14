use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("OAuth error: {0}")]
    OAuth(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn status_code(&self) -> u16 {
        match self {
            Error::Database(_) | Error::Internal(_) => 500,
            Error::Auth(_) | Error::Unauthorized(_) => 401,
            Error::Forbidden(_) => 403,
            Error::NotFound(_) => 404,
            Error::Validation(_) | Error::InvalidRequest(_) => 400,
            Error::RateLimitExceeded => 429,
            Error::OAuth(_) | Error::Platform(_) | Error::Config(_) => 500,
            Error::Other(_) => 500,
        }
    }
}
