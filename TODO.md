# Crosspost-RS TODO

**Last updated:** 2026-02-15
**Status:** Compiles clean, 25 tests passing, ~65% complete

---

## Done

- [x] Cargo workspace with 5 crates (core, auth, db, platforms, api)
- [x] Core domain types with serde, validator, Display/FromStr
- [x] Error types with HTTP status code mapping (12 variants)
- [x] Platform enum with 10 variants
- [x] Environment-based configuration
- [x] SurrealDB client with CRUD operations
- [x] Cache client (SurrealDB-based)
- [x] OAuth handler with URLs/scopes for all 10 platforms
- [x] OAuth code exchange flow
- [x] OAuth state/CSRF via cache (encodes user_id + tenant_id)
- [x] Token manager with refresh support
- [x] JWT authentication (HS256) with Argon2 password hashing
- [x] User registration and login endpoints
- [x] Auth middleware with Claims extraction
- [x] All 10 platform clients: Twitter, Facebook, Instagram, LinkedIn, YouTube, TikTok, Reddit, Twitch, Slack, Telegram
- [x] Platform trait (post, validate_token, platform_name)
- [x] Post creation handler with multi-platform dispatch
- [x] Account ownership checks on disconnect and post
- [x] Token refresh before platform API calls
- [x] Rate limiting (global per-endpoint: auth 20/min, write 5/sec, read 30/sec)
- [x] CORS middleware (needs restriction from Any)
- [x] Prelude modules on all library crates
- [x] 25 unit tests (core types, errors, JWT, password, rate limiter)
- [x] CI pipeline (cargo check, clippy, fmt, test)
- [x] Dockerfile + docker-compose.yml
- [x] JS artifacts cleaned up

---

## Critical Priority

### Fix Bugs
- [ ] Fix Telegram delimiter inconsistency (`:` in post vs `|` in validate)
- [ ] Fix hardcoded `Platform::Twitter` in error responses (posts.rs:55)
- [ ] Fetch real platform account info after OAuth (not placeholder IDs)

### Security
- [ ] Restrict CORS to configured origins (not `Any`)
- [ ] Per-user rate limiting (keyed by user_id from JWT)
- [ ] Sanitize error messages sent to clients (log full errors server-side)
- [ ] Validate JWT secret length >= 32 bytes in config

### Data Integrity
- [ ] Persist PlatformPost records to DB after successful posts
- [ ] Add unique index on user email in SurrealDB

---

## High Priority

### Feature Parity with Original JS Library
The original `@humanwhocodes/crosspost` supports features we're missing:

- [ ] **Media/Image upload** - Up to 4 images per post with alt text
  - Platform-specific upload flows (Twitter media API, Bluesky blob upload, Mastodon media endpoint, etc.)
  - MIME type detection and validation
  - Image dimension detection (for Bluesky aspect ratios)
- [ ] **Bluesky platform** - Full AT Protocol support with facet detection (links, mentions, hashtags)
- [ ] **Mastodon platform** - With media upload support
- [ ] **Discord platform** - Bot-based channel posting
- [ ] **Discord Webhook platform** - Webhook-based posting
- [ ] **Dev.to platform** - Article publishing
- [ ] **Nostr platform** - Decentralized posting via relays
- [ ] **Slack platform** - With file upload support (our Slack is hardcoded to #general)
- [ ] **Message length calculation** - Per-platform character counting algorithms
- [ ] **Post URL extraction** - Return URL of posted content from each platform
- [ ] **AbortSignal support** - Cancellable post operations
- [ ] **Strategy pattern** - Pluggable client-side posting (original is a library, not just SaaS)

### Library Mode
The original crosspost is a **library** you import and use directly. Our Rust version is only a SaaS API server. To match:
- [ ] Make `crosspost-platforms` usable standalone (already partially done via prelude)
- [ ] `Client` struct that accepts strategies and calls `post()` on all of them
- [ ] CLI binary for command-line posting
- [ ] MCP server mode (original supports this)

### Background Jobs
- [ ] Scheduled post processor (tokio interval or cron)
- [ ] Token refresh scheduler
- [ ] Failed post retry queue

### Testing
- [ ] Integration tests for API handlers (mock HTTP)
- [ ] Platform client tests with mock responses
- [ ] OAuth flow tests
- [ ] Database operation tests
- [ ] Test error paths, not just happy paths

---

## Medium Priority

### API Completeness
- [ ] Schedule management (list, update, cancel scheduled posts)
- [ ] Post history with pagination (currently hardcoded LIMIT 50)
- [ ] Return 201 Created for POST endpoints (REST compliance)
- [ ] Request body size limits
- [ ] Graceful shutdown

### Database
- [ ] Add TTL to cache entries (OAuth state, rate limits)
- [ ] Cascade delete for disconnected accounts
- [ ] Index definitions for common queries
- [ ] Transaction support for multi-platform post creation
- [ ] Migration system

### OAuth Hardening
- [ ] PKCE support (required by Twitter, recommended everywhere)
- [ ] Token encryption at rest
- [ ] Configurable Slack channel (not hardcoded #general)
- [ ] Cache LinkedIn author URN in ConnectedAccount

### Platform Improvements
- [ ] LinkedIn: allow visibility settings (not hardcoded PUBLIC)
- [ ] YouTube/TikTok: error on missing post ID instead of "unknown"
- [ ] All platforms: log actual API errors instead of "Unknown error"
- [ ] Return post URL from each platform response

---

## Low Priority

### Monitoring & Operations
- [ ] Request ID tracing (X-Request-ID header)
- [ ] Prometheus metrics endpoint
- [ ] Health check including DB connectivity
- [ ] Security headers (CSP, HSTS, X-Frame-Options)
- [ ] Audit logging for sensitive operations

### Additional Platforms (beyond original library)
- [ ] Threads (Meta)
- [ ] Pinterest

### Documentation
- [ ] Rustdoc for public APIs
- [ ] OpenAPI/Swagger spec
- [ ] Getting started guide

---

## Architecture Notes

```
crosspost-rs/
├── crates/
│   ├── core/       # Types, errors, config (shared by all)
│   ├── auth/       # JWT, OAuth2, password hashing, token management
│   ├── db/         # SurrealDB client, cache client
│   ├── platforms/  # Platform trait + 10 client implementations
│   └── api/        # Axum server, routes, handlers, middleware
├── Dockerfile      # Multi-stage build
├── docker-compose.yml
└── Cargo.toml      # Workspace root
```
