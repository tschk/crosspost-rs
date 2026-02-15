use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::{num::NonZeroU32, sync::Arc};

/// Shared rate limiter type (not keyed, global per-limiter instance)
pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a rate limiter with the given requests per second
pub fn create_rate_limiter(per_second: u32) -> SharedRateLimiter {
    let quota = Quota::per_second(NonZeroU32::new(per_second).expect("rate limit must be > 0"));
    Arc::new(RateLimiter::direct(quota))
}

/// Create a rate limiter with the given requests per minute
pub fn create_rate_limiter_per_minute(per_minute: u32) -> SharedRateLimiter {
    let quota = Quota::per_minute(NonZeroU32::new(per_minute).expect("rate limit must be > 0"));
    Arc::new(RateLimiter::direct(quota))
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    // Extract rate limiter from request extensions (set by layer)
    let limiter = request.extensions().get::<SharedRateLimiter>().cloned();

    if let Some(limiter) = limiter {
        match limiter.check() {
            Ok(_) => Ok(next.run(request).await),
            Err(_) => {
                let body = serde_json::json!({
                    "error": "Rate limit exceeded. Please try again later.",
                });
                Ok((StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response())
            }
        }
    } else {
        // No rate limiter configured, pass through
        Ok(next.run(request).await)
    }
}

/// Layer that injects a rate limiter into request extensions
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
                Err(_) => {
                    let body = serde_json::json!({
                        "error": "Rate limit exceeded. Please try again later.",
                    });
                    Ok((StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response())
                }
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
        // First request should succeed
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_create_rate_limiter_per_minute() {
        let limiter = create_rate_limiter_per_minute(60);
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_rate_limiter_exhaustion() {
        // Create a limiter that allows 1 request per second
        let limiter = create_rate_limiter(1);

        // First request should succeed
        assert!(limiter.check().is_ok());

        // Subsequent requests within the same second should fail
        // (governor uses a token bucket, so the second request may or may not succeed
        //  depending on timing - but a burst of many should eventually fail)
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
}
