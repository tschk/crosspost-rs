# Feature Parity: Original JS vs Rust Rewrite

**Original:** `@humanwhocodes/crosspost` v1.0.3 (JavaScript, Apache-2.0)
**Rewrite:** `crosspost-rs` (Rust, Polyform Shield 1.0.0)
**Date:** 2026-02-16

---

## Platform Support

| Platform | JS Original | Rust Rewrite | Notes |
|----------|:-----------:|:------------:|-------|
| Twitter/X | YES | YES | Both use API v2. JS uses `twitter-api-v2` npm package; Rust uses raw reqwest |
| Bluesky | YES | YES | Rust matches JS with full AT Protocol support (facets, blob upload, aspect ratios) |
| Mastodon | YES | YES | Rust supports media upload (up to 4) and custom instances via `token|host` |
| LinkedIn | YES | YES | JS uses access token directly; Rust uses OAuth2 flow. Neither handles media |
| Discord (Bot) | YES | YES | Rust supports bot token + channel posting + image upload |
| Discord (Webhook) | YES | YES | Rust supports webhook URL posting + image upload |
| Telegram | YES | YES | Both use Bot API. Rust uses HTML parse mode and supports photo upload |
| Dev.to | YES | YES | Rust supports article creation via API key + image appending |
| Nostr | YES | YES | Rust supports secp256k1 signing + WebSocket relays (no images, same as JS) |
| Slack | YES | YES | Rust supports configurable channel (default #general) and file upload |
| Facebook | NO | YES | Rust addition - Graph API posting |
| Instagram | NO | YES | Rust addition - Graph API posting |
| YouTube | NO | YES | Rust addition - Video/community posting |
| TikTok | NO | YES | Rust addition - Video publishing |
| Reddit | NO | YES | Rust addition - Submission API |
| Twitch | NO | YES | Rust addition - Chat announcements |

**Summary:** JS: 10 platforms | Rust: 16 platforms | **Overlap: 10** (All JS platforms supported)

---

## Core Features

| Feature | JS Original | Rust Rewrite | Notes |
|---------|:-----------:|:------------:|-------|
| Post text to multiple platforms | YES | YES | Both dispatch to multiple platforms in parallel |
| Image upload (up to 4) | YES | YES | Rust supports image upload for Bluesky, Mastodon, Discord, Dev.to, Slack |
| Image alt text | YES | YES | Rust passes alt text where supported (Bluesky, Mastodon, etc.) |
| Image aspect ratio detection | YES | YES | Rust detects aspect ratio for Bluesky using `image` crate |
| MIME type detection | YES | YES | Rust uses `infer` crate to detect MIME type from bytes |
| Message length validation | YES | YES | Rust has per-platform `max_message_length()` implementation |
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

### Bluesky (Both)
- AT Protocol session management
- Rich text facet detection: URLs, mentions (@handle), hashtags
- Blob upload for images with aspect ratio metadata
- Post URL construction

### Mastodon (Both)
- Media upload (up to 4 attachments)
- Custom host support (any Mastodon instance)
- Visibility settings implied by API

### Twitter (Both)
- JS: Uses `twitter-api-v2` npm package
- Rust: Raw reqwest to API v2, OAuth2 bearer token
- Rust: No media upload yet

### Discord (Both)
- Bot mode: token + channel ID, creates messages via REST API
- Webhook mode: fires payload to webhook URL
- Image upload as file attachment (multipart)

### Nostr (Both)
- secp256k1 key pair signing
- WebSocket relay connections
- NIP-01 event creation

### Dev.to (Both)
- Article creation via API key
- Markdown body content
- Published flag control

### Slack (Both)
- JS: Token + channel, supports file upload
- Rust: Token + configurable channel (defaults #general), supports file upload

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
1. **CLI binary** - JS has a CLI; Rust version is currently API-only.
2. **MCP server mode** - JS supports Model Context Protocol; Rust does not.
3. **AbortSignal equivalent** - Rust needs tokio CancellationToken integration for request cancellation.

### Nice to have for parity
4. **Twitter media upload** - JS supports it, Rust does not.

### Completed Parity Items
- **Image/media upload**: Implemented for Bluesky, Mastodon, Discord, Dev.to, Slack.
- **Bluesky support**: Fully implemented.
- **Mastodon support**: Fully implemented.
- **Discord support**: Fully implemented (Bot & Webhook).
- **Dev.to support**: Fully implemented.
- **Nostr support**: Fully implemented.
- **Slack support**: Fully implemented (including file uploads).
- **Message length validation**: Implemented per platform.

### Different by design (not gaps)
- JS is a library; Rust is a SaaS platform (both valid, different use cases)
- JS uses direct credentials; Rust uses OAuth2 flows
- Rust has persistence, multi-tenancy, scheduling that JS doesn't need
