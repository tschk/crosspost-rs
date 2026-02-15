# Crosspost-RS TODO

**Last updated:** 2026-02-15
**Status:** 86 tests passing, zero clippy warnings. Library crate complete, ready for import.

This is a **Rust library** rewrite of [`@humanwhocodes/crosspost`](https://github.com/humanwhocodes/crosspost) with additional platforms and improvements. The library is importable by other Rust projects and will also be usable as a CLI and MCP server.

---

## Phase 1: Library Architecture (Complete)

### Restructure Workspace

- [x] Create `crosspost` library crate alongside existing server crates
- [x] Create a `Client` struct that holds `Vec<Box<dyn Strategy>>` and orchestrates posting
  - `Client::new(strategies: Vec<Box<dyn Strategy>>)` - constructor
  - `Client::post(message, options)` - post to ALL strategies concurrently
  - `Client::post_to(entries)` - post different messages to specific strategies by ID
  - Uses `futures::future::join_all` with individual error catching
- [x] `Strategy` trait (matching JS library pattern)
  - `fn name(&self) -> &str` - display name
  - `fn id(&self) -> &str` - machine identifier
  - `fn max_message_length(&self) -> usize`
  - `fn calculate_message_length(&self, message: &str) -> usize`
  - `async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse>`
  - `fn get_url_from_response(&self, response: &PostResponse) -> Option<String>`
  - `async fn validate_credentials(&self) -> Result<bool>`
- [x] `PostOptions` struct with optional images
- [x] `PostToEntry` struct for selective posting
- [x] `PostResult` enum with `Success` and `Failure` variants
- [x] Typed credential structs for all 16 platforms
- [x] Credential validation in constructors (return `Result`, not panic)
- [x] Root-level re-exports: `use crosspost::{Client, BlueskyStrategy, PostResult, ...}`

### Environment Configuration

- [x] `CROSSPOST_DOTENV` support: `"1"` = `.env` in cwd, other value = file path
- [x] `Strategy::from_env()` constructor on each strategy
- [x] Platform-specific env var loading (matching JS naming where applicable)

### All 16 Strategy Implementations

- [x] TwitterStrategy - OAuth2 bearer, image upload, URL counting (23 chars)
- [x] BlueskyStrategy - AT Protocol, session auth, blob upload, facets, aspect ratios
- [x] MastodonStrategy - configurable host, media upload
- [x] LinkedInStrategy - image upload (3-step register/upload/post)
- [x] FacebookStrategy - Graph API, single + multi image upload
- [x] InstagramStrategy - Graph API image containers
- [x] DiscordStrategy - Bot token, multipart image upload
- [x] DiscordWebhookStrategy - Webhook URL, multipart image upload
- [x] TelegramStrategy - Bot API, sendPhoto
- [x] SlackStrategy - 3-step file upload, configurable channel
- [x] DevtoStrategy - API key, title/body split, base64 images in markdown
- [x] NostrStrategy - secp256k1 signing, WebSocket relay publishing
- [x] YouTubeStrategy - Data API community posts
- [x] TikTokStrategy - Content Posting API
- [x] RedditStrategy - Submission API
- [x] TwitchStrategy - Chat announcements API

---

## Phase 2: CLI Binary

- [ ] Create `crosspost-cli` crate with `clap`
- [ ] Platform selection flags: `--twitter/-t`, `--mastodon/-m`, `--bluesky/-b`, `--linkedin/-l`, `--discord/-d`, `--discord-webhook`, `--devto`, `--telegram`, `--slack/-s`, `--nostr/-n`, `--facebook`, `--instagram`, `--youtube`, `--tiktok`, `--reddit`, `--twitch`
- [ ] `--file <path>` - read message from a file
- [ ] `--image <path>` - attach an image (repeatable, max 4)
- [ ] `--image-alt <text>` - alt text for the image
- [ ] `--mcp` flag to start in MCP server mode
- [ ] Message as positional argument or via stdin
- [ ] Output: checkmark/cross per platform with URL or error
- [ ] Exit code: 0 if all succeeded, 1 if any failed

---

## Phase 3: MCP Server Mode

- [ ] Implement MCP server using Rust MCP SDK
- [ ] MCP Prompts: `crosspost`, `post-to-{strategy.id}`
- [ ] MCP Tools: `crosspost`, `list-services`, `post-to-social-media`, `check-message-length`, `calculate-message-length`, `resize-message`

---

## Phase 4: Platform Improvements

- [ ] Twitter: Switch to OAuth 1.0a User Context (matching JS library)
- [ ] LinkedIn: Cache person URN to avoid extra API call per post
- [ ] LinkedIn: Configurable visibility (PUBLIC, CONNECTIONS)
- [ ] Instagram: Support image upload via publicly hosted URL flow
- [ ] Mastodon: Support media focus points (matching JS library)
- [ ] Nostr: Support NIP-94 image attachments
- [ ] Telegram: Send all images as media group (not just first)
- [ ] All platforms: Ensure `get_url_from_response()` returns real URLs

---

## Phase 5: Core Library Features

- [ ] `tokio::CancellationToken` support for aborting in-flight posts
- [ ] Strategy-level errors include platform name and HTTP status

---

## Phase 6: Testing

- [x] 86 unit tests across workspace
- [x] Strategy constructor validation tests
- [x] Message length calculation tests per platform
- [ ] Client `post()` tests with mock strategies
- [ ] Client `post_to()` tests with selective targeting
- [ ] Error isolation tests (one strategy fails, others succeed)
- [ ] Image MIME detection edge case tests
- [ ] CLI integration tests
- [ ] MCP server tool response tests

---

## Phase 7: Optional SaaS API Layer

The existing server crates can remain for users who want a hosted SaaS:

- [ ] Move API server to `crosspost-server` crate (optional, not the main crate)
- [ ] Server imports and wraps the `crosspost` library
- [ ] Token encryption at rest
- [ ] Transaction support for multi-platform posts
- [ ] Migration system

---

## Phase 8: Documentation

- [ ] Rustdoc for all public types and methods
- [ ] Platform credential setup guide
- [ ] Migration guide from JS version
