use crate::server::auth::Claims;
use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::{num::NonZeroU32, sync::Arc};

/// Global rate limiter (for unauthenticated routes like auth endpoints)
pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Per-user keyed rate limiter (for authenticated routes)
pub type KeyedRateLimiter =
    Arc<RateLimiter<String, governor::state::keyed::DashMapStateStore<String>, DefaultClock>>;

/// Create a global rate limiter with the given requests per second
pub fn create_rate_limiter(per_second: u32) -> SharedRateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(per_second).expect("rate limit must be > 0"));
    Arc::new(RateLimiter::direct(quota))
}

/// Create a global rate limiter with the given requests per minute
pub fn create_rate_limiter_per_minute(per_minute: u32) -> SharedRateLimiter {
    let quota = Quota::per_minute(NonZeroU32::new(per_minute).expect("rate limit must be > 0"));
    Arc::new(RateLimiter::direct(quota))
}

/// Create a per-user keyed rate limiter with the given requests per second
pub fn create_keyed_rate_limiter(per_second: u32) -> KeyedRateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(per_second).expect("rate limit must be > 0"));
    Arc::new(RateLimiter::keyed(quota))
}

/// Create a per-user keyed rate limiter with the given requests per minute
pub fn create_keyed_rate_limiter_per_minute(per_minute: u32) -> KeyedRateLimiter {
    let quota = Quota::per_minute(NonZeroU32::new(per_minute).expect("rate limit must be > 0"));
    Arc::new(RateLimiter::keyed(quota))
}

fn rate_limit_response() -> Response {
    let body = serde_json::json!({
        "error": "Rate limit exceeded. Please try again later.",
    });
    (StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response()
}

/// Layer for global (not-keyed) rate limiting (public routes)
#[derive(Clone)]
pub struct RateLimitLayer {
    limiter: SharedRateLimiter,
}

impl RateLimitLayer {
    pub fn new(limiter: SharedRateLimiter) -> Self {
        Self { limiter }
    }
}

impl<S> tower::Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    limiter: SharedRateLimiter,
}

impl<S> tower::Service<Request> for RateLimitService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            match limiter.check() {
                Ok(_) => inner.call(request).await,
                Err(_) => Ok(rate_limit_response()),
            }
        })
    }
}

/// Layer for per-user (keyed) rate limiting (authenticated routes)
#[derive(Clone)]
pub struct KeyedRateLimitLayer {
    limiter: KeyedRateLimiter,
}

impl KeyedRateLimitLayer {
    pub fn new(limiter: KeyedRateLimiter) -> Self {
        Self { limiter }
    }
}

impl<S> tower::Layer<S> for KeyedRateLimitLayer {
    type Service = KeyedRateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        KeyedRateLimitService {
            inner,
            limiter: self.limiter.clone(),
        }
    }
}

#[derive(Clone)]
pub struct KeyedRateLimitService<S> {
    inner: S,
    limiter: KeyedRateLimiter,
}

impl<S> tower::Service<Request> for KeyedRateLimitService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let limiter = self.limiter.clone();
        let mut inner = self.inner.clone();

        // Extract user_id from Claims if available (set by auth middleware)
        let key = request
            .extensions()
            .get::<Claims>()
            .map(|c| c.sub.to_string())
            .unwrap_or_else(|| "anonymous".to_string());

        Box::pin(async move {
            match limiter.check_key(&key) {
                Ok(_) => inner.call(request).await,
                Err(_) => Ok(rate_limit_response()),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_rate_limiter() {
        let limiter = create_rate_limiter(10);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_create_rate_limiter_per_minute() {
        let limiter = create_rate_limiter_per_minute(60);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_rate_limiter_exhaustion() {
        let limiter = create_rate_limiter(1);
        assert!(limiter.check().is_ok());

        let mut rejected = false;
        for _ in 0..100 {
            if limiter.check().is_err() {
                rejected = true;
                break;
            }
        }
        assert!(
            rejected,
            "Rate limiter should reject requests after exhaustion"
        );
    }

    #[test]
    fn test_keyed_rate_limiter() {
        let limiter = create_keyed_rate_limiter(1);
        // Different keys should have independent limits
        assert!(limiter.check_key(&"user1".to_string()).is_ok());
        assert!(limiter.check_key(&"user2".to_string()).is_ok());
    }

    #[test]
    fn test_keyed_rate_limiter_per_user_exhaustion() {
        let limiter = create_keyed_rate_limiter(1);
        let key = "user1".to_string();

        assert!(limiter.check_key(&key).is_ok());

        let mut rejected = false;
        for _ in 0..100 {
            if limiter.check_key(&key).is_err() {
                rejected = true;
                break;
            }
        }
        assert!(
            rejected,
            "Per-user rate limiter should reject after exhaustion"
        );

        // Another user should still be able to make requests
        assert!(limiter.check_key(&"user2".to_string()).is_ok());
    }
}
