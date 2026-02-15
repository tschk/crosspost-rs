use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Message too long for {platform}: {length} characters (max {max})")]
    MessageTooLong {
        platform: String,
        length: usize,
        max: usize,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
