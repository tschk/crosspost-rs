use crate::strategy::Strategy;
use crate::types::{PostOptions, PostResult, PostToEntry};

/// Client that orchestrates posting to multiple strategies concurrently.
///
/// # Example
///
/// ```rust,no_run
/// use crosspost::{Client, BlueskyStrategy, BlueskyCredentials};
///
/// # async fn example() -> crosspost::Result<()> {
/// let client = Client::new(vec![
///     Box::new(BlueskyStrategy::new(BlueskyCredentials {
///         identifier: "user.bsky.social".into(),
///         password: "app-password".into(),
///         host: None,
///     })?),
/// ]);
///
/// let results = client.post("Hello from Rust!", None).await;
/// # Ok(())
/// # }
/// ```
pub struct Client {
    strategies: Vec<Box<dyn Strategy>>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field(
                "strategies",
                &self
                    .strategies
                    .iter()
                    .map(|s| s.id())
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Client {
    /// Create a new client with the given strategies.
    pub fn new(strategies: Vec<Box<dyn Strategy>>) -> Self {
        Self { strategies }
    }

    /// Post a message to ALL configured strategies concurrently.
    ///
    /// Each strategy is called independently - one failure does not affect others.
    /// Returns a `PostResult` for each strategy.
    pub async fn post(&self, message: &str, options: Option<&PostOptions>) -> Vec<PostResult> {
        let futures: Vec<_> = self
            .strategies
            .iter()
            .map(|strategy| {
                let name = strategy.name().to_string();
                async move {
                    // Check message length first
                    let length = strategy.calculate_message_length(message);
                    let max = strategy.max_message_length();
                    if length > max {
                        return PostResult::Failure {
                            name,
                            reason: format!(
                                "Message too long: {} characters (max {})",
                                length, max
                            ),
                        };
                    }

                    match strategy.post(message, options).await {
                        Ok(response) => {
                            let url = strategy.get_url_from_response(&response);
                            PostResult::Success {
                                name,
                                post_id: response.id,
                                url,
                            }
                        }
                        Err(e) => PostResult::Failure {
                            name,
                            reason: e.to_string(),
                        },
                    }
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }

    /// Post different messages to specific strategies by ID.
    ///
    /// Only strategies whose IDs match entries in the input will be called.
    /// Unmatched entries produce a `Failure` result.
    pub async fn post_to(&self, entries: &[PostToEntry]) -> Vec<PostResult> {
        let futures: Vec<_> = entries
            .iter()
            .map(|entry| {
                let strategy = self.strategies.iter().find(|s| s.id() == entry.strategy_id);

                async move {
                    let strategy = match strategy {
                        Some(s) => s,
                        None => {
                            return PostResult::Failure {
                                name: entry.strategy_id.clone(),
                                reason: format!(
                                    "No strategy found with id '{}'",
                                    entry.strategy_id
                                ),
                            };
                        }
                    };

                    let name = strategy.name().to_string();

                    // Check message length
                    let length = strategy.calculate_message_length(&entry.message);
                    let max = strategy.max_message_length();
                    if length > max {
                        return PostResult::Failure {
                            name,
                            reason: format!(
                                "Message too long: {} characters (max {})",
                                length, max
                            ),
                        };
                    }

                    let options = entry.images.as_ref().map(|images| PostOptions {
                        images: images.clone(),
                    });

                    match strategy.post(&entry.message, options.as_ref()).await {
                        Ok(response) => {
                            let url = strategy.get_url_from_response(&response);
                            PostResult::Success {
                                name,
                                post_id: response.id,
                                url,
                            }
                        }
                        Err(e) => PostResult::Failure {
                            name,
                            reason: e.to_string(),
                        },
                    }
                }
            })
            .collect();

        futures::future::join_all(futures).await
    }

    /// Get a reference to all configured strategies.
    pub fn strategies(&self) -> &[Box<dyn Strategy>] {
        &self.strategies
    }
}
