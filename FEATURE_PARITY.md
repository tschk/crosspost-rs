# Feature Parity: Original JS vs Rust Rewrite

**Original:** `@humanwhocodes/crosspost` v1.0.3 (JavaScript, Apache-2.0)
**Rewrite:** `crosspost-rs` (Rust, Polyform Shield 1.0.0)
**Date:** 2026-02-15

---

## Platform Support

| Platform | JS Original | Rust Rewrite | Notes |
|----------|:-----------:|:------------:|-------|
| Twitter/X | YES | YES | Both use API v2. JS uses `twitter-api-v2` npm package; Rust uses raw reqwest |
| Bluesky | YES | NO | JS has full AT Protocol: session auth, facet detection (links/mentions/hashtags), blob upload, aspect ratios |
| Mastodon | YES | NO | JS has media upload with focus points; Rust doesn't have this platform |
| LinkedIn | YES | YES | JS uses access token directly; Rust uses OAuth2 flow. Neither handles media |
| Discord (Bot) | YES | NO | JS supports bot token + channel posting |
| Discord (Webhook) | YES | NO | JS supports webhook URL posting |
| Telegram | YES | YES | Both use Bot API. JS uses botToken+chatId params; Rust has delimiter bug |
| Dev.to | YES | NO | JS publishes articles via API key |
| Nostr | YES | NO | JS uses secp256k1 signing + WebSocket relays (requires Node 22+) |
| Slack | YES | YES | Both post to channels. JS supports file upload; Rust hardcoded to #general |
| Facebook | NO | YES | Rust addition - Graph API posting |
| Instagram | NO | YES | Rust addition - Graph API posting |
| YouTube | NO | YES | Rust addition - Video/community posting |
| TikTok | NO | YES | Rust addition - Video publishing |
| Reddit | NO | YES | Rust addition - Submission API |
| Twitch | NO | YES | Rust addition - Chat announcements |

**Summary:** JS: 10 platforms | Rust: 10 platforms | **Overlap: 4** (Twitter, LinkedIn, Telegram, Slack)

---

## Core Features

| Feature | JS Original | Rust Rewrite | Notes |
|---------|:-----------:|:------------:|-------|
| Post text to multiple platforms | YES | YES | Both dispatch to multiple platforms in parallel |
| Image upload (up to 4) | YES | NO | JS supports PNG/JPEG/GIF with alt text per image |
| Image alt text | YES | NO | JS passes alt text per-image to each platform |
| Image aspect ratio detection | YES | NO | JS uses `image-size` for Bluesky aspect ratios |
| MIME type detection | YES | NO | JS detects from binary data |
| Message length validation | YES | NO | JS has per-platform `MAX_MESSAGE_LENGTH` and `calculateMessageLength()` |
| Post URL extraction | YES | PARTIAL | JS returns URL via `getUrlFromResponse()`; Rust returns `url: Option<String>` but many are None |
| AbortSignal/cancellation | YES | NO | JS supports `AbortSignal` for cancelling in-flight posts |
| Strategy pattern (pluggable) | YES | YES | JS: `Client` + `Strategy` interface; Rust: `Platform` trait |
| Per-message strategy targeting | YES | YES | JS: `postTo()` with `strategyId`; Rust: `account_ids` in request |
| Error handling (per-platform) | YES | YES | JS: `SuccessResponse`/`FailureResponse` per platform; Rust: `PlatformPostResult` per account |
| CLI binary | YES | NO | JS has full CLI with flags per platform |
| MCP server mode | YES | NO | JS can run as Model Context Protocol server for AI agents |
| .env file loading | YES | PARTIAL | JS: `CROSSPOST_DOTENV` env var; Rust: manual env loading |

---

## Authentication Model

| Aspect | JS Original | Rust Rewrite |
|--------|:-----------:|:------------:|
| Auth model | Direct credentials (tokens/keys passed in code) | OAuth2 + JWT (SaaS model) |
| User accounts | None - library usage | Yes - registration, login, JWT |
| Multi-tenant | No | Yes - tenant_id in JWT |
| OAuth flows | None (user provides tokens) | Full OAuth2 for all platforms |
| Token storage | None (user's responsibility) | SurrealDB with refresh |
| Password hashing | N/A | Argon2 |

The JS library is a **client-side library** where you bring your own credentials.
The Rust version is a **SaaS platform** that manages OAuth connections for users.

These are fundamentally different architectures. The JS approach is simpler for individual developers; the Rust approach is designed for multi-tenant business use.

---

## Platform-Specific Features

### Bluesky (JS only)
- AT Protocol session management (createSession, resolveHandle)
- Rich text facet detection: URLs, mentions (@handle), hashtags
- Blob upload for images with aspect ratio metadata
- Post URL construction from AT URI

### Mastodon (JS only)
- Media upload with FormData (up to 4 attachments)
- Focus point metadata on images
- Custom host support (any Mastodon instance)
- Visibility settings implied by API

### Twitter (both)
- JS: Uses `twitter-api-v2` npm package with OAuth 1.0a User Context
- JS: Media upload via package's built-in upload method
- Rust: Raw reqwest to API v2, OAuth2 bearer token
- Rust: No media upload

### Discord (JS only)
- Bot mode: token + channel ID, creates messages via REST API
- Webhook mode: fires payload to webhook URL
- Image upload as file attachment (multipart)

### Nostr (JS only)
- secp256k1 key pair signing (bech32 or hex format)
- WebSocket relay connections
- NIP-01 event creation with proper tags

### Dev.to (JS only)
- Article creation via API key
- Markdown body content
- Published flag control

### Slack (both)
- JS: Token + channel, supports file upload (multi-step: get upload URL, upload, complete)
- Rust: Token-based, hardcoded #general, no file upload

---

## What Rust Adds (Not in JS)

| Feature | Description |
|---------|-------------|
| **SaaS API server** | Full HTTP API with Axum, not just a library |
| **Multi-tenant** | Tenant isolation via JWT claims |
| **User management** | Registration, login, JWT tokens |
| **OAuth account management** | Connect/disconnect platform accounts |
| **Persistent storage** | SurrealDB for posts, accounts, schedules |
| **Rate limiting** | Governor-based per-endpoint limits |
| **Post scheduling** | Store scheduled posts (executor not yet built) |
| **6 extra platforms** | Facebook, Instagram, YouTube, TikTok, Reddit, Twitch |
| **CI/CD** | GitHub Actions pipeline |
| **Docker** | Multi-stage build with SurrealDB |

---

## Gap Summary

### Must have for parity
1. **Image/media upload** - This is the biggest gap. The JS library supports up to 4 images with alt text on every platform. The Rust version is text-only.
2. **Bluesky support** - Major platform in the JS library with rich features (facets, blob upload).
3. **Mastodon support** - Major platform, especially for the open-source community.
4. **Message length validation** - JS validates per-platform; Rust has a global 10,000 char limit.

### Nice to have for parity
5. **Discord support** (bot and/or webhook)
6. **Dev.to support**
7. **Nostr support**
8. **CLI binary**
9. **MCP server mode**
10. **AbortSignal equivalent** (tokio CancellationToken)

### Different by design (not gaps)
- JS is a library; Rust is a SaaS platform (both valid, different use cases)
- JS uses direct credentials; Rust uses OAuth2 flows
- Rust has persistence, multi-tenancy, scheduling that JS doesn't need
