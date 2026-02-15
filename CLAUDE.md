# AI Assistant Instructions for Crosspost-RS

## Project Overview

Crosspost-RS is a Rust library for cross-posting content to 16 social media platforms. It is a rewrite and expansion of the original JavaScript [crosspost](https://github.com/humanwhocodes/crosspost) library.

**Repository:** GraftAI-com/crosspost-rs
**License:** Polyform Shield 1.0.0

## Build & Run

```bash
cargo check                       # Check compilation
cargo build --release             # Build release
cargo test --workspace            # Run all tests (86 currently)
cargo clippy --workspace -- -D warnings  # Lint (must pass with zero warnings)
cargo fmt --all -- --check        # Format check
```

## Workspace Structure

```
crates/
├── crosspost/  # Main library crate - Strategy pattern, Client, 16 platforms
├── core/       # Shared types, errors, config (used by server crates)
├── auth/       # JWT, OAuth2, password hashing (server)
├── db/         # SurrealDB client + cache (server)
├── platforms/  # Platform trait + clients (server, older pattern)
└── api/        # Axum HTTP server, routes, handlers (server)
```

The `crosspost` crate is the standalone library. The other crates are for an optional SaaS server layer.

## Key Dependencies (crosspost crate)

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
- Edition 2021, resolver v2
- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- Use `thiserror` for error types
- The `crosspost` crate has its own `Error` and `Result` types (standalone, not dependent on `crosspost_core`)
- Server crates use `crosspost_core::Error` and `crosspost_core::Result`
- Async everywhere with `#[async_trait::async_trait]` for trait impls
- Strategies use `reqwest::Client` stored as a field
- Use `serde` derives on types that cross crate boundaries
- Never use `.unwrap()` in production code

### Error Handling
- `crosspost::Error` variants: `Platform(String)`, `Validation(String)`, `Config(String)`, `MessageTooLong { platform, length, max }`
- Map external errors with `.map_err(|e| Error::Platform(format!(...)))`
- Use `?` with `.map_err()` for conversions

### Strategy Pattern (crosspost crate)
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

### Platform Client Pattern (server crates, older)
Server crates use the `Platform` trait in `crosspost-platforms` with `access_token: &str` parameter and pipe-delimited multi-value tokens.

### Image Support
Images are passed via `PostOptions.images: Option<Vec<ImageEmbed>>`:
```rust
pub struct ImageEmbed {
    pub data: Vec<u8>,
    pub alt: Option<String>,
    pub mime_type: Option<String>,
}
```

Utility functions in `crates/crosspost/src/util/images.rs`:
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
- `FacebookCredentials { access_token, page_id: Option<String> }`
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

### API Handler Pattern (server crates)
Protected handlers use Axum extractors with Claims from JWT middleware:
```rust
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RequestType>,
) -> Result<Json<ResponseType>, AppError> { ... }
```

### SurrealDB Patterns (server crates)
- Use `("table", id.to_string())` tuples for record IDs
- `.content(owned_value)` not `.content(&borrowed)` (SurrealDB requires `'static`)
- `.create()` returns `Option<T>`, not `Vec<T>`

## Architecture Decisions

- **Strategy pattern** - credentials baked into struct at construction time (not passed per-call)
- **Concurrent posting** via `futures::future::join_all` with per-strategy error isolation
- **Standalone library** - `crosspost` crate has no dependency on server crates
- **Raw reqwest** for all platform APIs (no SDK crates) for consistency and control
- **Manual Nostr crypto** using secp256k1/sha2/bech32 (avoids heavy nostr-sdk dependency)
- **Message length validation** before platform dispatch (prevents wasted API calls)
- **Typed credentials** instead of pipe-delimited strings

## What NOT to Do

- Don't add dependencies without checking if an existing one covers the need
- Don't create new error types in the `crosspost` crate - extend `crosspost::Error` if needed
- Don't use `println!` - use `tracing::info!`, `tracing::error!`, etc.
- Don't use `.unwrap()` in production code
- When adding a new strategy, also:
  - Add credential struct to `crates/crosspost/src/types.rs`
  - Add module + re-export in `crates/crosspost/src/strategies/mod.rs`
  - Add re-export in `crates/crosspost/src/lib.rs`
  - Add tests for constructor validation, metadata, and message length

## Testing Guidelines

- Unit tests in the same file (`#[cfg(test)] mod tests`)
- Mock HTTP responses for strategy tests (don't hit real APIs)
- Use `tokio::test` for async tests
- Test error paths, not just happy paths
- New strategies should test: credential validation, metadata (id, name, max_message_length), and custom message length calculation
