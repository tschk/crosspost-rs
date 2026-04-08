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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_status_codes() {
        assert_eq!(Error::Database("db".into()).status_code(), 500);
        assert_eq!(Error::Internal("err".into()).status_code(), 500);
        assert_eq!(Error::Auth("auth".into()).status_code(), 401);
        assert_eq!(Error::Unauthorized("unauth".into()).status_code(), 401);
        assert_eq!(Error::Forbidden("forbidden".into()).status_code(), 403);
        assert_eq!(Error::NotFound("missing".into()).status_code(), 404);
        assert_eq!(Error::Validation("bad".into()).status_code(), 400);
        assert_eq!(Error::InvalidRequest("bad".into()).status_code(), 400);
        assert_eq!(Error::RateLimitExceeded.status_code(), 429);
        assert_eq!(Error::OAuth("oauth".into()).status_code(), 500);
        assert_eq!(Error::Platform("plat".into()).status_code(), 500);
        assert_eq!(Error::Config("cfg".into()).status_code(), 500);
    }

    #[test]
    fn test_error_display() {
        let err = Error::NotFound("user 123".to_string());
        assert_eq!(err.to_string(), "Not found: user 123");

        let err = Error::RateLimitExceeded;
        assert_eq!(err.to_string(), "Rate limit exceeded");
    }
}
