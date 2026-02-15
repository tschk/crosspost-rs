# AI Assistant Instructions for Crosspost-RS

## Project Overview

Crosspost-RS is a multi-tenant SaaS platform for cross-posting content to social media, written in Rust. It is a rewrite and expansion of the original JavaScript [crosspost](https://github.com/humanwhocodes/crosspost) library.

**Repository:** GraftAI-com/crosspost-rs
**License:** Polyform Shield 1.0.0

## Build & Run

```bash
cargo check                       # Check compilation
cargo build --release             # Build release binary
cargo run --bin crosspost-server  # Run the API server
cargo test --workspace            # Run all tests
cargo clippy --workspace -- -D warnings  # Lint (must pass with zero warnings)
cargo fmt --all -- --check        # Format check
docker-compose up -d              # Start with SurrealDB
```

## Workspace Structure

```
crates/
├── core/       # Shared types, errors, config - depended on by all other crates
├── auth/       # JWT, OAuth2, password hashing, token management
├── db/         # SurrealDB client + cache client
├── platforms/  # Platform trait + 16 client implementations + image utilities
└── api/        # Axum HTTP server, routes, handlers, middleware
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| tokio | Async runtime (full features) |
| axum | HTTP framework (macros, multipart) |
| surrealdb | Primary database (kv-rocksdb feature) |
| oauth2 | OAuth 2.0 client |
| reqwest | HTTP client for platform APIs |
| jsonwebtoken | JWT token generation and validation |
| argon2 | Password hashing |
| thiserror | Error derive macros |
| serde/serde_json | Serialization |
| chrono | Timestamps |
| uuid | Identifiers (v4) |
| validator | Request validation derives |
| governor | Rate limiting (available, not yet per-endpoint) |
| tower-http | CORS, tracing, request-id |
| tracing | Structured logging |
| image | Image dimension detection (Bluesky aspect ratios) |
| infer | MIME type detection from binary magic bytes |
| base64 | Image data encoding/decoding |
| secp256k1 | Nostr event signing (schnorr) |
| sha2 | SHA-256 hashing (Nostr event IDs) |
| bech32 | Nostr nsec1/note1 key encoding |
| hex | Hex encoding/decoding (Nostr keys, event IDs) |
| tokio-tungstenite | WebSocket (Nostr relay publishing) |
| futures-util | Async stream utilities (WebSocket sink) |

## Supported Platforms (16)

| Platform | Auth Method | Image Support | Max Length | Token Format |
|----------|-------------|---------------|------------|--------------|
| Twitter | OAuth2 | Planned | 280 (URLs=23) | Bearer token |
| Facebook | OAuth2 | Planned | 63,206 | Bearer token |
| Instagram | OAuth2 | Planned | 2,200 | Bearer token |
| LinkedIn | OAuth2 | Planned | 3,000 | Bearer token |
| YouTube | OAuth2 | No | 5,000 | Bearer token |
| TikTok | OAuth2 | No | 2,200 | Bearer token |
| Reddit | OAuth2 | No | 40,000 | Bearer token |
| Twitch | OAuth2 | No | 500 | Bearer token |
| Slack | OAuth2 | Yes (3-step upload) | 40,000 | `token\|channel_id` |
| Telegram | Bot API | Yes (sendPhoto) | 4,096 | `bot_token\|chat_id` |
| Bluesky | App password | Yes (blob upload) | 300 (URLs=27) | `identifier\|app_password` |
| Mastodon | OAuth2 | Yes (media upload) | 500 | `token\|host` |
| Discord | Bot token | Yes (multipart) | 2,000 | `bot_token\|channel_id` |
| Discord Webhook | Webhook URL | Yes (multipart) | 2,000 | Full webhook URL |
| Dev.to | API key | Yes (base64 markdown) | Unlimited | API key |
| Nostr | Private key | No | 280 | `privkey\|relay1,relay2` |

## Code Conventions

### Rust Style
- Edition 2021, resolver v2
- `cargo fmt` and `cargo clippy -- -D warnings` must pass
- Use `thiserror` for error types, not manual `impl Display`
- Use `crosspost_core::Result<T>` everywhere
- Use `crosspost_core::Error` variants for all errors
- Async everywhere with `#[async_trait::async_trait]` for trait impls
- Platform clients use `reqwest::Client` stored as a field
- Use `serde` derives on all types that cross crate boundaries
- Use `validator` derives on request types
- Add `impl Default` for any type that has `fn new() -> Self`

### Error Handling
- Map external errors to `Error::Database(...)`, `Error::Platform(...)`, etc.
- Use `?` with `.map_err()` for conversions
- Prefer `.map_err(Error::VariantName)` over `.map_err(|e| Error::VariantName(e))` when possible (clippy: redundant_closure)
- Never use `.unwrap()` in production code
- All errors map to HTTP status codes via `Error::status_code()`

### Platform Client Pattern
Every platform client follows this structure:

```rust
pub struct XxxClient {
    client: reqwest::Client,
}

impl XxxClient {
    pub fn new() -> Self {
        Self { client: reqwest::Client::new() }
    }
}

impl Default for XxxClient {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl Platform for XxxClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse>;
    async fn validate_token(&self, access_token: &str) -> Result<bool>;
    fn platform_name(&self) -> &'static str;
    fn max_message_length(&self) -> usize { ... }          // Override for platform limit
    fn calculate_message_length(&self, content: &str) -> usize { ... }  // Override if URLs count differently
}
```

### Platform Token Formats
Some platforms encode multiple values in the access_token using `|` as delimiter:
- **Telegram:** `bot_token|chat_id`
- **Bluesky:** `identifier|app_password`
- **Mastodon:** `access_token|host` (defaults to mastodon.social)
- **Discord Bot:** `bot_token|channel_id`
- **Discord Webhook:** full webhook URL (no delimiter)
- **Nostr:** `private_key|relay1,relay2,...`
- **Slack:** `token|channel_id` (defaults to #general)

### Image Support
Images are passed via `PostRequest.images: Option<Vec<ImageEmbed>>`:
```rust
pub struct ImageEmbed {
    pub data: Vec<u8>,          // Raw image bytes
    pub alt: Option<String>,     // Alt text
    pub mime_type: Option<String>, // image/png, image/jpeg, image/gif
}
```
API clients send `CreatePostRequest.images: Option<Vec<ImageData>>` with base64-encoded data, which the handler decodes into `ImageEmbed`.

Utility functions in `crates/platforms/src/util/images.rs`:
- `detect_mime_type(data)` - MIME detection via `infer`
- `image_dimensions(data)` - Width/height via `image`
- `validate_images(images)` - Type and count validation (max 4)

### API Handler Pattern
Protected handlers use Axum extractors with Claims from JWT middleware:

```rust
pub async fn handler(
    State(state): State<Arc<AppState>>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RequestType>,
) -> Result<Json<ResponseType>, AppError> {
    let user_id = claims.sub;
    let tenant_id = claims.tenant_id;
    // ...
}
```

### SurrealDB Patterns
- Use `("table", id.to_string())` tuples for record IDs
- `.content(owned_value)` not `.content(&borrowed)` (SurrealDB requires `'static`)
- `.create()` returns `Option<T>`, not `Vec<T>`
- `.select()` with tuple returns `Option<T>` for single records
- `.query()` with `.take(0)` returns `Vec<T>` for query results

### Authentication Flow
- Public routes: `/health`, `/auth/register`, `/auth/login`, `/auth/:platform/callback`
- Protected routes use `auth_middleware` which validates JWT Bearer tokens
- Claims (user_id, tenant_id, email) are stored in request extensions
- Handlers extract claims via `Extension(claims): Extension<Claims>`
- Non-OAuth platforms (Bluesky, Telegram, Nostr, Dev.to) store credentials directly

## Architecture Decisions

- **SurrealDB** as primary database (multi-model: document + graph + relational)
- **SurrealDB cache table** for OAuth state and rate limit counters
- **Axum** over Actix-Web for tower middleware ecosystem
- **Trait-based platform abstraction** - each platform implements `Platform` trait
- **JWT authentication** with Argon2 password hashing
- **Multi-tenant via JWT claims** - tenant_id in every token, verified in handlers
- **OAuth2 crate** for standardized OAuth flows across all platforms
- **Raw reqwest** for all platform APIs (no SDK crates) for consistency and control
- **Manual Nostr crypto** using secp256k1/sha2/bech32 (avoids heavy nostr-sdk dependency)
- **Message length validation** before platform dispatch (prevents wasted API calls)

## What NOT to Do

- Don't add new dependencies without checking if an existing one covers the need
- Don't create new error types - extend `crosspost_core::Error` if needed
- Don't use `println!` - use `tracing::info!`, `tracing::error!`, etc.
- Don't add platforms to the `Platform` enum without also adding:
  - OAuth URL/scope mappings in `crates/auth/src/oauth.rs`
  - Config field in `crates/core/src/config.rs` `OAuthConfig`
  - Match arm in `crates/api/src/handlers/auth.rs` `get_platform_oauth_config()`
  - Match arm in `crates/api/src/handlers/posts.rs` `create_post()`
  - Module + re-export in `crates/platforms/src/lib.rs`
- Don't change the workspace structure without good reason
- Don't borrow values passed to SurrealDB `.content()` - it requires owned data
- Don't use inconsistent token delimiters - always use `|` for multi-value tokens
- There are leftover JS files (src/, tests/, eslint.config.js, etc.) from the original library - ignore them

## Testing Guidelines

When adding tests:
- Unit tests in the same file (`#[cfg(test)] mod tests`)
- Integration tests in `crates/*/tests/`
- Mock HTTP responses for platform client tests (don't hit real APIs)
- Use `tokio::test` for async tests
- Test error paths, not just happy paths
- New platform clients should test: token parsing, message length calculation, max length
