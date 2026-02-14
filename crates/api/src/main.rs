use crosspost_api::run_server;
use crosspost_core::config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let config = AppConfig::from_env()?;

    // Run the server
    run_server(config).await?;

    Ok(())
}
