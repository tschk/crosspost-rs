use crate::strategy::Strategy;
use crate::types::{PostOptions, PostResult, PostToEntry};
use tokio_util::sync::CancellationToken;

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
                &self.strategies.iter().map(|s| s.id()).collect::<Vec<_>>(),
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
        self.post_with_cancel(message, options, None).await
    }

    /// Like [`Client::post`], but aborts in-flight work when `cancel` is triggered.
    ///
    /// Each strategy future is cancelled cooperatively: the underlying HTTP request may
    /// run until the runtime drops it after cancellation. Use [`CancellationToken::cancel`]
    /// from another task or signal handler.
    pub async fn post_with_cancel(
        &self,
        message: &str,
        options: Option<&PostOptions>,
        cancel: Option<&CancellationToken>,
    ) -> Vec<PostResult> {
        let futures: Vec<_> = self
            .strategies
            .iter()
            .map(|strategy| {
                let name = strategy.name().to_string();
                async move {
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

                    let post_fut = strategy.post(message, options);
                    let outcome = if let Some(ct) = cancel {
                        tokio::select! {
                            biased;
                            () = ct.cancelled() => {
                                Err("Post cancelled".to_string())
                            }
                            r = post_fut => Ok(r),
                        }
                    } else {
                        Ok(post_fut.await)
                    };

                    match outcome {
                        Err(reason) => PostResult::Failure { name, reason },
                        Ok(Err(e)) => PostResult::Failure {
                            name,
                            reason: e.to_string(),
                        },
                        Ok(Ok(response)) => {
                            let url = strategy.get_url_from_response(&response);
                            PostResult::Success {
                                name,
                                post_id: response.id,
                                url,
                            }
                        }
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
        self.post_to_with_cancel(entries, None).await
    }

    /// Like [`Client::post_to`], but aborts in-flight strategy posts when `cancel` is triggered.
    pub async fn post_to_with_cancel(
        &self,
        entries: &[PostToEntry],
        cancel: Option<&CancellationToken>,
    ) -> Vec<PostResult> {
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

                    let post_options = entry.images.as_ref().map(|images| PostOptions {
                        images: images.clone(),
                    });

                    let post_fut = strategy.post(&entry.message, post_options.as_ref());
                    let outcome = if let Some(ct) = cancel {
                        tokio::select! {
                            biased;
                            () = ct.cancelled() => {
                                Err("Post cancelled".to_string())
                            }
                            r = post_fut => Ok(r),
                        }
                    } else {
                        Ok(post_fut.await)
                    };

                    match outcome {
                        Err(reason) => PostResult::Failure { name, reason },
                        Ok(Err(e)) => PostResult::Failure {
                            name,
                            reason: e.to_string(),
                        },
                        Ok(Ok(response)) => {
                            let url = strategy.get_url_from_response(&response);
                            PostResult::Success {
                                name,
                                post_id: response.id,
                                url,
                            }
                        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;
    use crate::strategy::PostResponse;
    use std::time::Duration;

    /// A mock strategy for testing the Client orchestrator.
    struct MockStrategy {
        strategy_id: &'static str,
        strategy_name: &'static str,
        max_length: usize,
        should_fail: bool,
        delay_post: Option<Duration>,
    }

    impl MockStrategy {
        fn success(id: &'static str, name: &'static str) -> Self {
            Self {
                strategy_id: id,
                strategy_name: name,
                max_length: 280,
                should_fail: false,
                delay_post: None,
            }
        }

        fn failing(id: &'static str, name: &'static str) -> Self {
            Self {
                strategy_id: id,
                strategy_name: name,
                max_length: 280,
                should_fail: true,
                delay_post: None,
            }
        }

        fn slow(id: &'static str, name: &'static str, delay: Duration) -> Self {
            Self {
                strategy_id: id,
                strategy_name: name,
                max_length: 280,
                should_fail: false,
                delay_post: Some(delay),
            }
        }
    }

    #[async_trait::async_trait]
    impl Strategy for MockStrategy {
        fn name(&self) -> &str {
            self.strategy_name
        }
        fn id(&self) -> &str {
            self.strategy_id
        }
        fn max_message_length(&self) -> usize {
            self.max_length
        }
        async fn post(
            &self,
            _message: &str,
            _options: Option<&PostOptions>,
        ) -> crate::error::Result<PostResponse> {
            if let Some(d) = self.delay_post {
                tokio::time::sleep(d).await;
            }
            if self.should_fail {
                Err(Error::Platform("Mock API error".to_string()))
            } else {
                Ok(PostResponse {
                    id: format!("{}-post-123", self.strategy_id),
                    url: Some(format!("https://example.com/{}/123", self.strategy_id)),
                })
            }
        }
        async fn validate_credentials(&self) -> crate::error::Result<bool> {
            Ok(!self.should_fail)
        }
    }

    #[tokio::test]
    async fn test_post_to_all_strategies() {
        let client = Client::new(vec![
            Box::new(MockStrategy::success("mock1", "Mock One")),
            Box::new(MockStrategy::success("mock2", "Mock Two")),
        ]);

        let results = client.post("Hello!", None).await;
        assert_eq!(results.len(), 2);

        for result in &results {
            match result {
                PostResult::Success { name, post_id, url } => {
                    assert!(!name.is_empty());
                    assert!(!post_id.is_empty());
                    assert!(url.is_some());
                }
                PostResult::Failure { .. } => panic!("Expected success"),
            }
        }
    }

    #[tokio::test]
    async fn test_post_with_cancel_aborts() {
        let client = Client::new(vec![Box::new(MockStrategy::slow(
            "slow",
            "Slow",
            Duration::from_secs(30),
        ))]);
        let token = CancellationToken::new();
        let child = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            child.cancel();
        });

        let results = client.post_with_cancel("Hello!", None, Some(&token)).await;
        assert_eq!(results.len(), 1);
        match &results[0] {
            PostResult::Failure { reason, .. } => assert_eq!(reason, "Post cancelled"),
            PostResult::Success { .. } => panic!("expected cancellation"),
        }
    }

    #[tokio::test]
    async fn test_post_to_with_cancel_aborts() {
        let client = Client::new(vec![Box::new(MockStrategy::slow(
            "slow",
            "Slow",
            Duration::from_secs(30),
        ))]);
        let token = CancellationToken::new();
        let child = token.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            child.cancel();
        });

        let entries = vec![PostToEntry {
            strategy_id: "slow".to_string(),
            message: "Hi".to_string(),
            images: None,
        }];
        let results = client.post_to_with_cancel(&entries, Some(&token)).await;
        assert_eq!(results.len(), 1);
        match &results[0] {
            PostResult::Failure { reason, .. } => assert_eq!(reason, "Post cancelled"),
            PostResult::Success { .. } => panic!("expected cancellation"),
        }
    }

    #[tokio::test]
    async fn test_post_error_isolation() {
        let client = Client::new(vec![
            Box::new(MockStrategy::success("good", "Good")),
            Box::new(MockStrategy::failing("bad", "Bad")),
        ]);

        let results = client.post("Hello!", None).await;
        assert_eq!(results.len(), 2);

        let successes: Vec<_> = results
            .iter()
            .filter(|r| matches!(r, PostResult::Success { .. }))
            .collect();
        let failures: Vec<_> = results
            .iter()
            .filter(|r| matches!(r, PostResult::Failure { .. }))
            .collect();

        assert_eq!(successes.len(), 1);
        assert_eq!(failures.len(), 1);

        if let PostResult::Failure { reason, .. } = &failures[0] {
            assert!(reason.contains("Mock API error"));
        }
    }

    #[tokio::test]
    async fn test_post_message_too_long() {
        let client = Client::new(vec![Box::new(MockStrategy::success("short", "Short"))]);

        let long_message = "x".repeat(300);
        let results = client.post(&long_message, None).await;
        assert_eq!(results.len(), 1);

        match &results[0] {
            PostResult::Failure { reason, .. } => {
                assert!(reason.contains("Message too long"));
                assert!(reason.contains("300"));
                assert!(reason.contains("280"));
            }
            PostResult::Success { .. } => panic!("Expected failure for long message"),
        }
    }

    #[tokio::test]
    async fn test_post_to_specific_strategies() {
        let client = Client::new(vec![
            Box::new(MockStrategy::success("alpha", "Alpha")),
            Box::new(MockStrategy::success("beta", "Beta")),
        ]);

        let entries = vec![PostToEntry {
            strategy_id: "alpha".to_string(),
            message: "Hello Alpha!".to_string(),
            images: None,
        }];

        let results = client.post_to(&entries).await;
        assert_eq!(results.len(), 1);

        match &results[0] {
            PostResult::Success { name, .. } => assert_eq!(name, "Alpha"),
            PostResult::Failure { .. } => panic!("Expected success"),
        }
    }

    #[tokio::test]
    async fn test_post_to_unknown_strategy() {
        let client = Client::new(vec![Box::new(MockStrategy::success("alpha", "Alpha"))]);

        let entries = vec![PostToEntry {
            strategy_id: "nonexistent".to_string(),
            message: "Hello!".to_string(),
            images: None,
        }];

        let results = client.post_to(&entries).await;
        assert_eq!(results.len(), 1);

        match &results[0] {
            PostResult::Failure { name, reason } => {
                assert_eq!(name, "nonexistent");
                assert!(reason.contains("No strategy found"));
            }
            PostResult::Success { .. } => panic!("Expected failure"),
        }
    }

    #[tokio::test]
    async fn test_empty_client() {
        let client = Client::new(vec![]);
        let results = client.post("Hello!", None).await;
        assert!(results.is_empty());
    }

    #[test]
    fn test_client_debug() {
        let client = Client::new(vec![
            Box::new(MockStrategy::success("alpha", "Alpha")),
            Box::new(MockStrategy::success("beta", "Beta")),
        ]);
        let debug = format!("{:?}", client);
        assert!(debug.contains("alpha"));
        assert!(debug.contains("beta"));
    }

    #[test]
    fn test_strategies_accessor() {
        let client = Client::new(vec![
            Box::new(MockStrategy::success("a", "A")),
            Box::new(MockStrategy::success("b", "B")),
        ]);
        assert_eq!(client.strategies().len(), 2);
        assert_eq!(client.strategies()[0].id(), "a");
        assert_eq!(client.strategies()[1].id(), "b");
    }
}
