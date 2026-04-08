use thiserror::Error;

/// Errors that can occur when using the crosspost library.
#[derive(Error, Debug)]
pub enum Error {
    /// An error returned by a platform's API.
    #[error("Platform error: {0}")]
    Platform(String),

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

/// A specialized `Result` type for crosspost operations.
pub type Result<T> = std::result::Result<T, Error>;
