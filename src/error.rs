use thiserror::Error;

/// Errors that can occur when using the crosspost library.
#[derive(Error, Debug)]
pub enum Error {
    /// An error returned by a platform's API.
    #[error("Platform error: {0}")]
    Platform(String),

    /// A non-success HTTP response from a platform API.
    #[error("{platform}: HTTP {status}: {details}")]
    PlatformHttp {
        /// Platform display name (e.g. "Bluesky").
        platform: String,
        /// HTTP status code.
        status: u16,
        /// Response body or other detail text.
        details: String,
    },

    /// A validation error (e.g., invalid image type, too many images).
    #[error("Validation error: {0}")]
    Validation(String),

    /// A configuration error (e.g., missing environment variable).
    #[error("Configuration error: {0}")]
    Config(String),

    /// The message exceeds the platform's maximum length.
    #[error("Message too long for {platform}: {length} characters (max {max})")]
    MessageTooLong {
        /// Platform display name.
        platform: String,
        /// Calculated message length.
        length: usize,
        /// Platform's maximum allowed length.
        max: usize,
    },
}

impl Error {
    /// Build a [`Error::PlatformHttp`] from a display name and HTTP status.
    pub fn platform_http(
        platform: impl Into<String>,
        status: reqwest::StatusCode,
        details: impl Into<String>,
    ) -> Self {
        Error::PlatformHttp {
            platform: platform.into(),
            status: status.as_u16(),
            details: details.into(),
        }
    }
}

/// Map a failed HTTP [`reqwest::Response`] to [`Error::PlatformHttp`] after reading the body.
pub(crate) async fn platform_response_error(platform: &str, response: reqwest::Response) -> Error {
    let status = response.status();
    let details = response
        .text()
        .await
        .unwrap_or_else(|_| "Unknown error".to_string());
    Error::platform_http(platform, status, details)
}

/// A specialized `Result` type for crosspost operations.
pub type Result<T> = std::result::Result<T, Error>;
