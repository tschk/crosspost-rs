# Crosspost-RS TODO

**Last updated:** 2026-02-15
**Status:** Foundation complete, compilation broken, ~35% implemented

---

## Blocking: Fix Compilation

- [ ] **Fix `crates/db/src/rocksdb_client.rs`** - 10 type errors from SurrealDB API misuse
  - `db.set()` returns `()`, not `Option<String>`
  - `db.select()` returns `Vec<_>`, not `Option<String>`
  - `db.delete()` returns `Vec<_>`, not `Option<String>`
  - Options: fix the type annotations, or replace with actual RocksDB for caching

---

## Done

- [x] Cargo workspace with 5 crates (core, auth, db, platforms, api)
- [x] Core domain types: Tenant, User, ConnectedAccount, Post, PlatformPost, ScheduledPost, PostStatus
- [x] Error types with HTTP status code mapping (12 variants)
- [x] Platform enum with 10 variants + Display/FromStr/as_str
- [x] Request/response types with validator derives
- [x] Environment-based configuration
- [x] SurrealDB client with CRUD operations
- [x] OAuth handler with URLs and scopes for all 10 platforms
- [x] OAuth code exchange flow
- [x] Token manager structure
- [x] Platform trait (post, validate_token, platform_name)
- [x] Twitter client: posting via API v2, token validation
- [x] Facebook client: posting via Graph API v18.0, token validation
- [x] Instagram client: posting via Graph API, token validation
- [x] Axum server with tracing
- [x] Route tree: health, auth, accounts, post, schedule
- [x] Tenant isolation middleware (header extraction)
- [x] AppError with status code mapping
- [x] OAuth connect/callback handlers
- [x] Post creation handler with multi-platform dispatch
- [x] Dockerfile with multi-stage build
- [x] docker-compose.yml with SurrealDB
- [x] .env.example with all config vars

---

## High Priority

### Authentication (no user identity system exists)
- [ ] JWT token generation and signing (RS256 or HS256)
- [ ] JWT validation middleware for Axum
- [ ] User registration endpoint
- [ ] Login/logout endpoints
- [ ] Password hashing with Argon2 (dependency already present)
- [ ] Tenant context extraction from JWT (replace X-Tenant-ID header)
- [ ] Database query scoping by tenant_id

### Fix and Wire Up Existing Dependencies
- [ ] CORS configuration (tower-http cors feature is present but unused)
- [ ] Rate limiting middleware (governor is in deps but unused)
- [ ] Request body size limits

### Remaining Platform Clients (7 of 10)
Each needs: post(), validate_token(), platform_name()
- [ ] LinkedIn - Organization/member posting via REST API
- [ ] YouTube - Community posts / video uploads
- [ ] TikTok - Video publishing API
- [ ] Reddit - Submission API
- [ ] Twitch - Chat announcements
- [ ] Slack - Channel posting via Web API
- [ ] Telegram - Bot API (not OAuth - already handled correctly in auth crate)

### CI/CD
- [ ] GitHub Actions workflow: `cargo check`, `cargo clippy`, `cargo test`
- [ ] Automated build on PR
- [ ] Dependency audit (`cargo audit`)

---

## Medium Priority

### Testing
- [ ] Unit tests for core types (Platform enum, error status codes)
- [ ] Unit tests for OAuth handler (URL generation, scope mapping)
- [ ] Unit tests for platform clients (mock HTTP responses)
- [ ] Integration tests for API handlers
- [ ] Integration tests for database operations

### OAuth Hardening
- [ ] PKCE support (required by Twitter, recommended everywhere)
- [ ] State/CSRF validation on callback (state is generated but not verified)
- [ ] Token encryption at rest
- [ ] Token refresh background job

### Database
- [ ] Fix or replace rocksdb_client.rs (use actual RocksDB or fix SurrealDB usage)
- [ ] Migration system
- [ ] Connection pooling
- [ ] Index definitions for common queries
- [ ] Transaction support for multi-platform post creation

### API Completion
- [ ] Account management endpoints (list, get, delete, update)
- [ ] Post history endpoints with pagination and filtering
- [ ] Schedule management endpoints (list, update, cancel)
- [ ] Graceful shutdown
- [ ] Request ID middleware (tower-http request-id feature present)

### Media Support
- [ ] Image upload handling in post endpoint
- [ ] Platform-specific media upload flows (Twitter media upload API, etc.)
- [ ] File size and format validation
- [ ] Instagram two-step container creation flow

---

## Low Priority

### Background Jobs
- [ ] Scheduled post processor (cron or tokio interval)
- [ ] Token refresh scheduler
- [ ] Failed post retry queue

### Monitoring
- [ ] Prometheus metrics endpoint
- [ ] Structured logging improvements
- [ ] Health check for database connectivity

### Security Hardening
- [ ] Security headers middleware (CSP, HSTS, X-Frame-Options)
- [ ] Audit logging for sensitive operations
- [ ] API key management for service-to-service auth

### Additional Platforms
- [ ] Mastodon
- [ ] Bluesky
- [ ] Threads
- [ ] Discord
- [ ] Discord Webhooks

### Documentation
- [ ] Rustdoc for public APIs
- [ ] OpenAPI/Swagger spec generation
- [ ] Getting started guide

---

## Known Issues

1. **Project does not compile** - rocksdb_client.rs has SurrealDB type mismatches
2. **No authentication** - user_id is `Uuid::new_v4()` in all handlers
3. **Tenant isolation is header-only** - not validated against any credential
4. **OAuth state not verified** - CSRF token generated but never checked on callback
5. **Instagram posting simplified** - doesn't use the required two-step container creation
6. **JS artifacts remain** - eslint.config.js, prettier.config.js, tsconfig.json, src/, tests/ from original JS library

---

## Architecture Notes

```
crosspost-rs/
├── crates/
│   ├── core/       # Types, errors, config (shared by all crates)
│   ├── auth/       # OAuth2 flows, token management
│   ├── db/         # SurrealDB client, cache client
│   ├── platforms/  # Platform trait + implementations (Twitter, FB, IG, ...)
│   └── api/        # Axum server, routes, handlers, middleware
├── Dockerfile      # Multi-stage build
├── docker-compose.yml
└── Cargo.toml      # Workspace root
```

**Key dependencies:** Tokio, Axum, SurrealDB, oauth2, reqwest, thiserror, serde, chrono, uuid, validator, governor, argon2
