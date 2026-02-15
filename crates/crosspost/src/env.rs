use std::env;

/// Load environment variables from a .env file if CROSSPOST_DOTENV is set.
///
/// - If `CROSSPOST_DOTENV` is `"1"`, loads `.env` from the current directory.
/// - If `CROSSPOST_DOTENV` is any other value, treats it as a file path to load.
/// - If `CROSSPOST_DOTENV` is not set, does nothing.
pub fn load_dotenv() {
    if let Ok(val) = env::var("CROSSPOST_DOTENV") {
        if val == "1" {
            let _ = dotenvy::dotenv();
        } else {
            let _ = dotenvy::from_filename(&val);
        }
    }
}

/// Helper to get a required environment variable.
pub(crate) fn required_env(name: &str) -> crate::Result<String> {
    env::var(name).map_err(|_| {
        crate::Error::Config(format!("Missing required environment variable: {}", name))
    })
}

/// Helper to get an optional environment variable.
pub(crate) fn optional_env(name: &str) -> Option<String> {
    env::var(name).ok()
}
