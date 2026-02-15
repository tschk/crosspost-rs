# Crosspost-RS Code Audit

**Audit Date:** 2026-02-15 (updated)
**Previous Audits:** 2026-02-14 (Copilot), 2026-02-15 (Claude initial)
**Repository:** GraftAI-com/crosspost-rs
**Branch:** copilot/rewrite-crosspost-library-in-rust

---

## Executive Summary

The Rust rewrite compiles cleanly with **zero warnings** and passes **25 tests**. Since the last audit, compilation was fixed, JWT auth was added, CORS/rate limiting were wired up, all 10 platform clients were implemented, and JS artifacts were removed. The project is approximately **65% complete** as a SaaS platform, but has **significant feature gaps** compared to the original JS library (see Feature Parity section).

### Compilation Status: PASSES
- `cargo check` - clean
- `cargo clippy -- -D warnings` - zero warnings
- `cargo fmt --all -- --check` - clean
- `cargo test --workspace` - 25 tests pass

---

## Critical Issues

### 1. OAuth Account Info Uses Placeholders (auth.rs:113)
**File:** `crates/api/src/handlers/auth.rs`
After OAuth callback, `platform_account_id` and `platform_account_name` are hardcoded:
```rust
let platform_account_id = format!("{}_{}", platform_str, user_id);
let platform_account_name = format!("{} account", platform_str);
```
Should fetch real account info from platform APIs after token exchange.

### 2. PlatformPost Records Never Persisted
**Files:** `crates/api/src/handlers/posts.rs`, `crates/db/src/surrealdb_client.rs`
No `create_platform_post()` DB method exists. Successful post results are returned in the API response but never stored. No post history/audit trail.

### 3. CORS Allows All Origins (lib.rs:32-35)
**File:** `crates/api/src/lib.rs`
```rust
let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);
```
Should be configurable from environment, restricted to specific origins.

### 4. Telegram Token Delimiter Inconsistency (telegram.rs)
**File:** `crates/platforms/src/telegram.rs`
`post()` splits on `:` (line 63) but `validate_token()` splits on `|` (line 137). One will always fail.

### 5. Hardcoded Platform in Error Response (posts.rs:55)
**File:** `crates/api/src/handlers/posts.rs`
All failed posts report `Platform::Twitter` regardless of actual platform:
```rust
platform: crosspost_core::Platform::Twitter,
```

---

## High Issues

### 6. Rate Limiting is Global, Not Per-User
**File:** `crates/api/src/rate_limit.rs`
Rate limiters are `NotKeyed` - shared across all users. One user can exhaust limits for everyone. Should use keyed rate limiting by user_id from JWT claims.

### 7. No JWT Secret Minimum Length Validation
**File:** `crates/core/src/config.rs`
JWT secret loaded from env with no length check. HS256 needs at least 32 bytes.

### 8. Scheduled Posts Not Functional
**File:** `crates/api/src/handlers/posts.rs`
`schedule_post()` stores the record but no background job processor exists. Posts will never actually fire.

### 9. Sensitive Error Messages Exposed (posts.rs:167)
Internal error strings (DB URLs, stack details) leak to API clients via:
```rust
error_message: Some(e.to_string()),
```

### 10. Dead Code in OAuth/TokenManager
**Files:** `crates/auth/src/oauth.rs:10`, `crates/auth/src/token_manager.rs:11`
```rust
#[allow(dead_code)]
db: Arc<SurrealDbClient>,
```
These `db` fields are never used, indicating incomplete implementation.

### 11. No Email Verification
Registration accepts any email without sending a verification email.

---

## Medium Issues

### 12. Slack Channel Hardcoded to #general (slack.rs:43)
Users can't choose which channel to post to.

### 13. LinkedIn Fetches Profile on Every Post (linkedin.rs:71-95)
Extra API call per post to get author URN. Should cache in ConnectedAccount.

### 14. No Pagination for List Posts (surrealdb_client.rs)
Hardcoded `LIMIT 50` with no offset/cursor parameters.

### 15. No Cascade Delete for Disconnected Accounts
`delete_connected_account()` leaves orphaned PlatformPost/ScheduledPost records.

### 16. YouTube/TikTok Return "unknown" Post IDs on Parse Failure
Instead of erroring, returns `"unknown"` which can't be tracked.

### 17. Cache Has No TTL (cache_client.rs)
OAuth state tokens and rate limit counters stored indefinitely. No expiry mechanism.

### 18. No Unique Constraint on User Email (surrealdb_client.rs)
Tables are SCHEMALESS with no indexes. Race condition could create duplicate emails.

---

## Low Issues

### 19. All Create Operations Return 200, Not 201
Non-REST-compliant. POST endpoints should return `StatusCode::CREATED`.

### 20. No Request ID Tracing
`tower_http::trace` configured but no X-Request-ID propagation.

### 21. LinkedIn Visibility Hardcoded to PUBLIC
Users can't choose post visibility.

### 22. No Connection Pooling for SurrealDB
Single `Surreal<Any>` instance.

### 23. Platform Client Error Masking
All clients use `.unwrap_or_else(|_| "Unknown error".to_string())`, hiding actual API errors.

---

## What's Working Well

- Clean workspace architecture with proper crate boundaries
- All 10 platform clients implemented with real API calls
- JWT authentication with Argon2 password hashing
- OAuth2 flows for all platforms
- Rate limiting infrastructure (needs per-user keying)
- Proper error types with `thiserror` and HTTP status mapping
- Parameterized DB queries (no injection)
- No `println!()` - proper `tracing` usage
- 25 unit tests passing
- CI pipeline with check/clippy/fmt/test

---

## Code Quality Metrics

| Metric | Score | Notes |
|--------|-------|-------|
| Architecture | 9/10 | Clean crate separation, good traits |
| Type Safety | 9/10 | Excellent Rust type system usage |
| Error Handling | 7/10 | Good patterns, leaks internals to clients |
| Implementation | 7/10 | All 10 platforms, auth works, DB works |
| Testing | 4/10 | 25 unit tests, no integration tests, no platform mocks |
| Security | 5/10 | Auth exists but CORS open, no per-user rate limits |
| **Overall** | **7/10** | Solid foundation, needs hardening |

---

**Total Issues: 23** (5 Critical, 6 High, 6 Medium, 5 Low)
