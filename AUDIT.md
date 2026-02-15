# Crosspost-RS Code Audit

**Audit Date:** 2026-02-15
**Previous Audit:** 2026-02-14 (GitHub Copilot)
**Re-auditor:** Claude
**Repository:** GraftAI-com/crosspost-rs
**Branch:** copilot/rewrite-crosspost-library-in-rust

---

## Executive Summary

The Rust rewrite foundation has been established with **~1,767 lines of code** across 5 crates in a Cargo workspace. The architecture is well-designed with clean separation of concerns. However, the **project does not currently compile** due to SurrealDB API type mismatches in the cache client, and several features described in the README are aspirational rather than implemented.

Actual completion is closer to **35%** when measured by working code (vs the previous audit's 20% estimate, which incorrectly marked several implemented features as stubs).

### Compilation Status: FAILS

10 compilation errors in `crates/db/src/rocksdb_client.rs` due to SurrealDB API type mismatches. All other crates compile individually but the workspace build fails because `crosspost-api` depends on `crosspost-db`.

---

## Corrections to Previous Audit

The 2026-02-14 Copilot audit contained several inaccuracies:

| Claim | Reality |
|-------|---------|
| "All post implementations are unimplemented!()" | **False.** Twitter, Facebook, Instagram all have real API integration code |
| "create_oauth_client has match arms with unimplemented!()" | **False.** All 10 platforms have OAuth URL/scope configurations |
| "Zero unimplemented!() macros" | **Correct** - there are none in the codebase |
| "Auth crate 40% complete" | **More like 80%.** OAuth handler is functional for all platforms |
| "Platform clients 20% complete" | **More like 50%.** 3 platforms have full post + validate implementations |
| "No PKCE implementation" | Correct, but PKCE is an enhancement, not a blocker |

---

## Detailed Analysis by Crate

### 1. Core Crate (`crates/core`)

**Completion: 95%** | **Compiles: Yes** | **Lines: ~324**

| File | Lines | Status |
|------|-------|--------|
| `src/lib.rs` | 6 | Module re-exports |
| `src/error.rs` | 61 | Complete - 12 error variants with status codes |
| `src/types.rs` | 202 | Complete - All domain types |
| `src/config.rs` | 55 | Complete - Environment-based config |

**What works:**
- 12 error variants with HTTP status code mapping via `thiserror`
- Platform enum with 10 variants, `Display`, `FromStr`, `as_str()`
- Domain types: Tenant, User, ConnectedAccount, Post, PlatformPost, ScheduledPost
- Request/response types with `validator` derive macros
- OAuth types (OAuthAuthorizationResponse, OAuthCallbackQuery)

**Gaps:**
- No `PostStatus::Cancelled` variant
- Config could use builder pattern
- No platform capabilities enum

---

### 2. Database Crate (`crates/db`)

**Completion: 50%** | **Compiles: NO** | **Lines: ~370**

| File | Lines | Status |
|------|-------|--------|
| `src/lib.rs` | 14 | Database trait definition |
| `src/surrealdb_client.rs` | 267 | Mostly complete |
| `src/rocksdb_client.rs` | 89 | **BROKEN** - Type errors |

**Critical Issue:** `rocksdb_client.rs` is misnamed - it uses SurrealDB, not RocksDB. The SurrealDB API calls have type mismatches:
- `db.set()` returns `()`, code expects `Option<String>`
- `db.select()` returns `Vec<_>`, code expects `Option<String>`
- `db.delete()` returns `Vec<_>`, code expects `Option<String>`

**10 compilation errors** prevent the entire workspace from building.

**What works (in surrealdb_client.rs):**
- Database trait with `init()` and `health_check()`
- CRUD operations for tenants, users, connected accounts
- Post creation and querying
- Scheduled post operations
- SurrealDB namespace/database initialization

**Gaps:**
- No connection pooling
- No retry logic
- No transaction support
- No migration system
- No indexes defined
- Cache client needs to be rewritten (either fix SurrealDB usage or switch to actual RocksDB)

---

### 3. Auth Crate (`crates/auth`)

**Completion: 80%** | **Compiles: Yes** | **Lines: ~250**

| File | Lines | Status |
|------|-------|--------|
| `src/lib.rs` | 5 | Module exports |
| `src/oauth.rs` | 142 | **Fully implemented** |
| `src/token_manager.rs` | 103 | Partially implemented |

**What works:**
- OAuth client creation for all 10 platforms (not stubs!)
- Authorization URL generation with per-platform scopes
- Code exchange for access tokens
- Platform-specific OAuth URLs for: Twitter, Facebook, Instagram, LinkedIn, YouTube, TikTok, Reddit, Twitch, Slack
- Telegram correctly returns an error (uses Bot API, not OAuth2)
- Per-platform scope definitions

**Gaps:**
- No PKCE support
- No state/CSRF validation (state is generated but not verified on callback)
- Token refresh partially implemented
- No token encryption at rest

---

### 4. Platforms Crate (`crates/platforms`)

**Completion: 50%** | **Compiles: Yes** | **Lines: ~291**

| File | Lines | Status |
|------|-------|--------|
| `src/lib.rs` | 6 | Module exports |
| `src/platform_trait.rs` | 27 | Complete |
| `src/twitter.rs` | 89 | **Fully implemented** |
| `src/facebook.rs` | 83 | **Fully implemented** |
| `src/instagram.rs` | 86 | **Fully implemented** |

**What works:**
- Platform trait with `post()`, `validate_token()`, `platform_name()`
- **Twitter**: Full posting via API v2, token validation via `/users/me`
- **Facebook**: Full posting via Graph API v18.0, token validation
- **Instagram**: Full posting via Graph API (media endpoint), token validation

**Gaps:**
- No media/image upload support (text-only posting)
- No retry logic or rate limit handling in clients
- 7 platforms not implemented: LinkedIn, YouTube, TikTok, Reddit, Twitch, Slack, Telegram
- No platform-specific error types
- Instagram posting is simplified (doesn't handle the two-step container creation flow)

---

### 5. API Crate (`crates/api`)

**Completion: 60%** | **Compiles: Yes** (if db compiled) | **Lines: ~532**

| File | Lines | Status |
|------|-------|--------|
| `src/main.rs` | 13 | Entry point |
| `src/lib.rs` | 45 | Server setup with tracing |
| `src/state.rs` | 35 | AppState with Arc |
| `src/routes.rs` | 38 | Route definitions |
| `src/middleware.rs` | 48 | Tenant isolation + AppError |
| `src/handlers/mod.rs` | 4 | Module exports |
| `src/handlers/health.rs` | 10 | Complete |
| `src/handlers/auth.rs` | 159 | OAuth flow handlers |
| `src/handlers/accounts.rs` | 16 | Stub |
| `src/handlers/posts.rs` | 165 | Post/schedule handlers |

**What works:**
- Axum server with tracing subscriber
- Route tree: `/health`, `/auth/{platform}/connect`, `/auth/{platform}/callback`, `/accounts`, `/post`, `/schedule`
- AppError with proper HTTP status codes from core Error
- Tenant isolation middleware (extracts X-Tenant-ID header)
- OAuth connect/callback handler logic
- Post creation handler with multi-platform dispatch

**Gaps:**
- No JWT authentication (user_id is `Uuid::new_v4()` placeholder)
- Tenant ID extracted from header but not validated against JWT
- No CORS configuration (tower-http cors feature is in deps but unused)
- No rate limiting middleware (governor is in deps but unused)
- No request body size limits
- Account handlers are stubs
- No graceful shutdown

---

## Security Assessment

### Critical

1. **No authentication** - All handlers use random UUID as user_id
2. **Tenant isolation is header-only** - X-Tenant-ID not validated against any credential
3. **OAuth tokens stored in plaintext** - No encryption at rest
4. **No CSRF validation** - OAuth state parameter generated but not verified on callback

### High

5. **No CORS configuration** - Dependency present but not wired up
6. **No rate limiting** - Governor dependency present but not wired up
7. **No request body size limits** - Axum defaults only

### Medium

8. **No input sanitization** beyond validator macros
9. **No security headers** (CSP, HSTS, etc.)
10. **No audit logging**

---

## Infrastructure

### Docker (Complete)
- Multi-stage Dockerfile with cargo-chef for caching
- docker-compose.yml with SurrealDB service
- Environment variable configuration via .env.example

### CI/CD (Missing)
- No GitHub Actions workflows for Rust (only JS remnants)
- No automated testing, linting, or security scanning

---

## Code Quality Metrics

| Metric | Score | Notes |
|--------|-------|-------|
| Architecture | 8/10 | Clean crate boundaries, good trait design |
| Type Safety | 9/10 | Excellent use of Rust's type system |
| Error Handling | 8/10 | Comprehensive error types with thiserror |
| Implementation | 5/10 | 3/10 platforms, DB broken, no auth |
| Documentation | 3/10 | Minimal rustdoc, good README |
| Testing | 0/10 | Zero tests |
| Security | 2/10 | Critical gaps in auth and encryption |
| **Overall** | **5/10** | Solid foundation, needs completion |

---

## Blocking Issues (Fix First)

1. **Fix compilation** - `crates/db/src/rocksdb_client.rs` has 10 type errors
2. **Implement authentication** - No user identity system exists
3. **Wire up CORS** - Required for any frontend integration

## Recommended Next Steps

1. Fix the `rocksdb_client.rs` type errors (or replace with actual RocksDB)
2. Add JWT authentication middleware
3. Wire up CORS and rate limiting (deps already present)
4. Add CI with `cargo check`, `cargo test`, `cargo clippy`
5. Implement remaining 7 platform clients
6. Add unit tests for core types and platform clients

---

**Audit completed by:** Claude
**Date:** 2026-02-15
**Next audit recommended:** After compilation is fixed and auth is implemented
