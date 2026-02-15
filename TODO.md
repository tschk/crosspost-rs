# Crosspost-RS TODO

**Last updated:** 2026-02-15
**Status:** Compiles clean, 48 tests passing, ~95% complete

---

## Done

- [x] Cargo workspace with 5 crates (core, auth, db, platforms, api)
- [x] Core domain types with serde, validator, Display/FromStr
- [x] Error types with HTTP status code mapping (12 variants)
- [x] Platform enum with 16 variants
- [x] Environment-based configuration
- [x] SurrealDB client with CRUD operations
- [x] Cache client (SurrealDB-based) with TTL support
- [x] OAuth handler with URLs/scopes for all platforms
- [x] OAuth code exchange flow with PKCE support (Twitter)
- [x] OAuth state/CSRF via cache (encodes user_id + tenant_id, 10-minute TTL)
- [x] Token manager with refresh support
- [x] Token refresh background scheduler (proactive, every 5 minutes)
- [x] JWT authentication (HS256) with Argon2 password hashing
- [x] JWT secret minimum length validation (>= 32 bytes)
- [x] User registration and login endpoints
- [x] Auth middleware with Claims extraction
- [x] All 16 platform clients implemented:
  - [x] Twitter, Facebook, Instagram, LinkedIn, YouTube, TikTok, Reddit, Twitch, Slack, Telegram (original 10)
  - [x] Bluesky (AT Protocol, session auth, blob upload, facet detection for URLs/mentions/hashtags, aspect ratios)
  - [x] Mastodon (configurable host, media upload)
  - [x] Discord Bot (REST API, multipart image upload)
  - [x] Discord Webhook (webhook URL, multipart image upload)
  - [x] Dev.to (API key auth, title/body split, base64 images in markdown)
  - [x] Nostr (secp256k1 schnorr signing, NIP-01 events, WebSocket relay publishing with OK/NOTICE response handling)
- [x] Platform trait with post, validate_token, platform_name, max_message_length, calculate_message_length
- [x] Post creation handler with multi-platform dispatch (all 16 platforms)
- [x] Account ownership checks on disconnect and post
- [x] Token refresh before platform API calls
- [x] Rate limiting: global for auth routes, per-user keyed for read/write routes
- [x] CORS middleware (configurable origins from environment, restrictive by default)
- [x] Prelude modules on all library crates
- [x] Image support types (ImageEmbed, ImageData)
- [x] Image utility module (MIME detection via infer, dimensions via image, validation, compression)
- [x] Image upload for 11 platforms (Twitter, Facebook, LinkedIn, Bluesky, Mastodon, Discord, Discord Webhook, Slack, Telegram, Dev.to)
- [x] Message length validation for all 16 platforms
- [x] Custom URL-aware length calculation (Twitter URLs=23, Bluesky URLs=27)
- [x] Pre-post message length validation for Twitter, Bluesky, Nostr
- [x] Fixed Telegram delimiter consistency (both post and validate use `|`)
- [x] Fixed Slack configurable channel (token|channel_id, defaults to #general)
- [x] Fixed hardcoded Platform::Twitter in error responses (uses account's actual platform)
- [x] Fixed YouTube/TikTok/Twitch/Slack post ID error handling (proper errors instead of "unknown" fallbacks)
- [x] Fetch real platform account info after OAuth callback (all OAuth platforms)
- [x] Direct connect endpoint for non-OAuth platforms (Bluesky, Telegram, Nostr, Dev.to, Discord Webhook)
- [x] Credential validation before storing non-OAuth accounts
- [x] PlatformPost records persisted to DB after successful posts
- [x] Unique index on user email in SurrealDB
- [x] DB indexes for common queries (users, accounts, posts, scheduled posts, platform posts)
- [x] Cascade delete for disconnected accounts (cleans up platform_posts, scheduled_posts)
- [x] Post history with pagination (limit/offset, max 100)
- [x] Schedule management (create, list, update, cancel scheduled posts)
- [x] Scheduled post processor (tokio interval, every 30 seconds)
- [x] Sanitize error messages (5xx errors return generic message, full error logged server-side)
- [x] Security headers (X-Content-Type-Options, X-Frame-Options, X-XSS-Protection, HSTS)
- [x] Request ID tracing (X-Request-ID header, auto-generated UUID, propagated in responses)
- [x] Request body size limits (10MB)
- [x] Graceful shutdown (SIGTERM + Ctrl+C)
- [x] Health check with DB connectivity verification
- [x] Removed dead_code warnings from OAuth/TokenManager
- [x] Return 201 Created for POST endpoints
- [x] 48 unit tests (core types, errors, JWT, password, rate limiter incl. per-user keyed, token parsing, message length, event signing)
- [x] CI pipeline (cargo check, clippy, fmt, test)
- [x] Dockerfile + docker-compose.yml
- [x] JS artifacts cleaned up

---

## Remaining Items

### Medium Priority

#### Database
- [ ] Transaction support for multi-platform post creation
- [ ] Migration system

#### OAuth Hardening
- [ ] Token encryption at rest
- [ ] Cache LinkedIn author URN in ConnectedAccount (avoid extra API call per post)

#### Platform Improvements
- [ ] LinkedIn: Allow visibility settings (not hardcoded PUBLIC) - requires PostRequest API changes
- [ ] Instagram: Image upload support (requires publicly hosted URLs, complex flow)
- [ ] Mastodon: Configurable instance URL in OAuth flow (currently hardcoded mastodon.social)

---

### Low Priority

#### Monitoring & Operations
- [ ] Prometheus metrics endpoint
- [ ] Audit logging for sensitive operations

#### Library Mode
- [ ] Make `crosspost-platforms` usable standalone (partially done via prelude)
- [ ] `Client` struct that accepts strategies and calls `post()` on all of them
- [ ] CLI binary for command-line posting
- [ ] MCP server mode (original supports this)

#### Additional Platforms (beyond original library)
- [ ] Threads (Meta)
- [ ] Pinterest

#### Documentation
- [ ] Rustdoc for public APIs
- [ ] OpenAPI/Swagger spec
- [ ] Getting started guide

#### Testing
- [ ] Integration tests for API handlers (mock HTTP)
- [ ] Platform client tests with mock responses
- [ ] OAuth flow tests
- [ ] Database operation tests

---

## Architecture Notes

```
crosspost-rs/
├── crates/
│   ├── core/       # Types, errors, config (shared by all)
│   ├── auth/       # JWT, OAuth2 (with PKCE), password hashing, token management
│   ├── db/         # SurrealDB client, cache client (with TTL)
│   ├── platforms/  # Platform trait + 16 client implementations
│   │   ├── src/
│   │   │   ├── twitter.rs        # OAuth2+PKCE, URLs=23 chars, image upload
│   │   │   ├── facebook.rs       # OAuth2, image upload (single + multi)
│   │   │   ├── instagram.rs      # OAuth2
│   │   │   ├── linkedin.rs       # OAuth2, 3-step image upload
│   │   │   ├── youtube.rs        # OAuth2
│   │   │   ├── tiktok.rs         # OAuth2
│   │   │   ├── reddit.rs         # OAuth2
│   │   │   ├── twitch.rs         # OAuth2 (needs client_id)
│   │   │   ├── slack.rs          # OAuth2, token|channel, 3-step file upload
│   │   │   ├── telegram.rs       # Bot API, token|chat_id, sendPhoto
│   │   │   ├── bluesky.rs        # AT Protocol, app password, blob upload, facets (URL+mention+hashtag)
│   │   │   ├── mastodon.rs       # OAuth2, token|host, media upload
│   │   │   ├── discord.rs        # Bot token, token|channel, multipart
│   │   │   ├── discord_webhook.rs # Webhook URL, multipart
│   │   │   ├── devto.rs          # API key, title/body markdown
│   │   │   ├── nostr.rs          # Private key, secp256k1, WebSocket relays (with OK wait)
│   │   │   ├── platform_trait.rs # Platform trait + ImageEmbed + PostRequest/Response
│   │   │   └── util/
│   │   │       └── images.rs     # MIME detection, dimensions, validation, compression
│   │   └── Cargo.toml
│   └── api/        # Axum server, routes, handlers, middleware, scheduler
├── Dockerfile      # Multi-stage build
├── docker-compose.yml
└── Cargo.toml      # Workspace root
```
