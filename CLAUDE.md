# AI Assistant Instructions for Crosspost-RS

## Project Overview

Crosspost-RS is a Rust library for cross-posting content to 16 social media platforms. It is a rewrite and expansion of the original JavaScript [crosspost](https://github.com/humanwhocodes/crosspost) library.

**Repository:** GraftAI-com/crosspost-rs
**License:** Polyform Shield 1.0.0

## Build & Run

```bash
cargo check                                          # Check library compilation
cargo build --release                                # Build library release
cargo build --features server                        # Build with server
cargo test                                           # Run library tests (118 tests)
cargo test --features server                         # Run all tests (141 tests)
cargo clippy --all-targets -- -D warnings            # Lint library
cargo clippy --all-targets --features server -- -D warnings  # Lint everything
cargo fmt --all -- --check                           # Format check
```

## Crate Structure

This is a **single crate** with an optional `server` feature flag:

```
src/
├── lib.rs          # Library root (re-exports all public types)
├── client.rs       # Client orchestrator (post to all/selective strategies)
├── error.rs        # Library Error and Result types
├── strategy.rs     # Strategy trait + PostResponse
├── types.rs        # Credential structs, ImageEmbed, PostOptions, PostResult
├── env.rs          # Environment variable helpers
├── util/
│   └── images.rs   # Image processing (MIME, dimensions, compression)
├── strategies/     # 16 platform strategy implementations
│   ├── mod.rs
│   ├── twitter.rs, bluesky.rs, mastodon.rs, linkedin.rs, ...
│   └── (one file per platform)
├── server/         # Optional SaaS server layer (behind "server" feature)
│   ├── mod.rs
│   ├── core/       # Shared server types, errors, config
│   ├── auth/       # JWT, OAuth2, password hashing
│   ├── db/         # SurrealDB client + cache
│   └── api/        # Axum HTTP server, routes, handlers, rate limiting
└── bin/
    └── server.rs   # Server binary entrypoint (requires "server" feature)
```

## Key Dependencies (library)

| Crate | Purpose |
|-------|---------|
| tokio | Async runtime |
| reqwest | HTTP client for platform APIs |
| serde/serde_json | Serialization |
| thiserror | Error derive macros |
| async-trait | Async trait support |
| futures | Concurrent posting (join_all) |
| dotenvy | .env file loading |
| image | Image dimension detection |
| infer | MIME type detection from magic bytes |
| base64 | Image data encoding |
| mozjpeg/oxipng | Image compression |
| secp256k1/sha2/hex/bech32 | Nostr crypto |
| tokio-tungstenite | WebSocket (Nostr relays) |

## Supported Platforms (16)

| Platform | Strategy | Auth | Images | Max Length |
|----------|----------|------|--------|------------|
| Twitter | `TwitterStrategy` | Bearer token | Yes | 280 (URLs=23) |
| Bluesky | `BlueskyStrategy` | App password | Yes (blob) | 300 (URLs=27) |
| Mastodon | `MastodonStrategy` | Bearer token | Yes (media) | 500 |
| LinkedIn | `LinkedInStrategy` | Bearer token | Yes (3-step) | 3,000 |
| Facebook | `FacebookStrategy` | Bearer token | Yes (multi) | 63,206 |
| Instagram | `InstagramStrategy` | Bearer token | Yes | 2,200 |
| Discord Bot | `DiscordStrategy` | Bot token | Yes (multipart) | 2,000 |
| Discord Webhook | `DiscordWebhookStrategy` | Webhook URL | Yes (multipart) | 2,000 |
| Telegram | `TelegramStrategy` | Bot API | Yes (sendPhoto) | 4,096 |
| Slack | `SlackStrategy` | Bot token | Yes (3-step) | 40,000 |
| Dev.to | `DevtoStrategy` | API key | Yes (base64 md) | Unlimited |
| Nostr | `NostrStrategy` | Private key | No | 280 |
| YouTube | `YouTubeStrategy` | Bearer token | No | 5,000 |
| TikTok | `TikTokStrategy` | Bearer token | No | 2,200 |
| Reddit | `RedditStrategy` | Bearer token | No | 40,000 |
| Twitch | `TwitchStrategy` | Bearer token | No | 500 |

## Code Conventions

### Rust Style
- Edition 2021
- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- Use `thiserror` for error types
- Library uses `crate::Error` and `crate::Result` (standalone)
- Server code uses `crate::server::core::Error` and `crate::server::core::Result`
- Async everywhere with `#[async_trait::async_trait]` for trait impls
- Strategies use `reqwest::Client` stored as a field
- Use `serde` derives on types that cross boundaries
- Never use `.unwrap()` in production code

### Error Handling
- `crosspost::Error` variants: `Platform(String)`, `Validation(String)`, `Config(String)`, `MessageTooLong { platform, length, max }`
- Map external errors with `.map_err(|e| Error::Platform(format!(...)))`
- Use `?` with `.map_err()` for conversions

### Strategy Pattern
Every strategy follows this structure:

```rust
pub struct XxxStrategy {
    client: reqwest::Client,
    credentials: XxxCredentials,
}

impl XxxStrategy {
    pub fn new(credentials: XxxCredentials) -> Result<Self> {
        // Validate required fields, return Err on empty
        Ok(Self { client: reqwest::Client::new(), credentials })
    }

    pub fn from_env() -> Result<Self> {
        Self::new(XxxCredentials {
            // Read from env vars
        })
    }
}

#[async_trait::async_trait]
impl Strategy for XxxStrategy {
    fn name(&self) -> &str { "Display Name" }
    fn id(&self) -> &str { "machine_id" }
    fn max_message_length(&self) -> usize { ... }
    fn calculate_message_length(&self, message: &str) -> usize { ... }  // Override if URLs count differently
    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse>;
    async fn validate_credentials(&self) -> Result<bool>;
}
```

### Image Support
Images are passed via `PostOptions.images: Vec<ImageEmbed>`:
```rust
pub struct ImageEmbed {
    pub data: Vec<u8>,
    pub alt: Option<String>,
    pub mime_type: Option<String>,
}
```

Utility functions in `src/util/images.rs`:
- `detect_mime_type(data)` - MIME detection via `infer`
- `image_dimensions(data)` - Width/height via `image`
- `validate_images(images)` - Type and count validation (max 4)
- `compress_jpeg(data, quality)` - JPEG compression via mozjpeg
- `compress_png(data)` - PNG compression via oxipng

### Credential Types
Each platform has a typed credential struct:
- `TwitterCredentials { access_token }`
- `BlueskyCredentials { identifier, password, host: Option<String> }`
- `MastodonCredentials { access_token, host }`
- `LinkedInCredentials { access_token }`
- `FacebookCredentials { access_token }`
- `InstagramCredentials { access_token }`
- `DiscordCredentials { bot_token, channel_id }`
- `DiscordWebhookCredentials { webhook_url }`
- `TelegramCredentials { bot_token, chat_id }`
- `SlackCredentials { bot_token, channel: Option<String> }`
- `DevtoCredentials { api_key }`
- `NostrCredentials { private_key, relays: Vec<String> }`
- `YouTubeCredentials { access_token }`
- `TikTokCredentials { access_token }`
- `RedditCredentials { access_token, subreddit: Option<String> }`
- `TwitchCredentials { access_token, client_id }`

### Server Handler Pattern (behind `server` feature)
Protected handlers use Axum extractors with Claims from JWT middleware:
```rust
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RequestType>,
) -> Result<Json<ResponseType>, AppError> { ... }
```

Server handlers use `create_strategy_for_account()` to map `ConnectedAccount` (with pipe-delimited tokens) to typed Strategy instances.

### SurrealDB Patterns (server)
- Use `("table", id.to_string())` tuples for record IDs
- `.content(owned_value)` not `.content(&borrowed)` (SurrealDB requires `'static`)
- `.create()` returns `Option<T>`, not `Vec<T>`

## Architecture Decisions

- **Single crate** - library at root, server behind `server` feature flag
- **Strategy pattern** - credentials baked into struct at construction time (not passed per-call)
- **Concurrent posting** via `futures::future::join_all` with per-strategy error isolation
- **Raw reqwest** for all platform APIs (no SDK crates) for consistency and control
- **Manual Nostr crypto** using secp256k1/sha2/bech32 (avoids heavy nostr-sdk dependency)
- **Message length validation** before platform dispatch (prevents wasted API calls)
- **Typed credentials** instead of pipe-delimited strings (library); server maps pipe-delimited DB tokens to typed credentials

## What NOT to Do

- Don't add dependencies without checking if an existing one covers the need
- Don't create new error types - extend `crosspost::Error` if needed
- Don't use `println!` - use `tracing::info!`, `tracing::error!`, etc.
- Don't use `.unwrap()` in production code
- When adding a new strategy, also:
  - Add credential struct to `src/types.rs`
  - Add module + re-export in `src/strategies/mod.rs`
  - Add re-export in `src/lib.rs`
  - Add tests for constructor validation, metadata, and message length

## Testing Guidelines

- Unit tests in the same file (`#[cfg(test)] mod tests`)
- Mock HTTP responses for strategy tests (don't hit real APIs)
- Use `tokio::test` for async tests
- Test error paths, not just happy paths
- New strategies should test: credential validation, metadata (id, name, max_message_length), and custom message length calculation
