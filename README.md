# crosspost

**Cross-post messages to multiple social media platforms from Rust.**

A Rust rewrite and expansion of the [`@humanwhocodes/crosspost`](https://github.com/humanwhocodes/crosspost) JavaScript library. Supports 16 platforms with typed credentials, concurrent posting, and per-platform error isolation.

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org)

---

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
crosspost = { git = "https://github.com/GraftAI-com/crosspost-rs.git" }
tokio = { version = "1", features = ["full"] }
```

```rust
use crosspost::{Client, BlueskyStrategy, BlueskyCredentials, MastodonStrategy, MastodonCredentials, PostResult};

#[tokio::main]
async fn main() -> crosspost::Result<()> {
    let client = Client::new(vec![
        Box::new(BlueskyStrategy::new(BlueskyCredentials {
            identifier: "user.bsky.social".into(),
            password: "app-password".into(),
            host: None,
        })?),
        Box::new(MastodonStrategy::new(MastodonCredentials {
            access_token: "your-token".into(),
            host: "mastodon.social".into(),
        })?),
    ]);

    let results = client.post("Hello from Rust!", None).await;

    for result in &results {
        match result {
            PostResult::Success { name, url, .. } => {
                println!("Posted to {}: {}", name, url.as_deref().unwrap_or("ok"));
            }
            PostResult::Failure { name, reason } => {
                println!("Failed on {}: {}", name, reason);
            }
        }
    }

    Ok(())
}
```

### Load credentials from environment

Each strategy has a `from_env()` constructor that reads platform-specific environment variables:

```rust
use crosspost::{Client, BlueskyStrategy, TwitterStrategy};

let client = Client::new(vec![
    Box::new(BlueskyStrategy::from_env()?),
    Box::new(TwitterStrategy::from_env()?),
]);
```

Set `CROSSPOST_DOTENV=1` to auto-load a `.env` file, or `CROSSPOST_DOTENV=/path/to/.env` for a custom path.

### Post to specific platforms

```rust
use crosspost::PostToEntry;

let results = client.post_to(&[
    PostToEntry {
        strategy_id: "bluesky".into(),
        message: "Short post for Bluesky".into(),
        images: None,
    },
    PostToEntry {
        strategy_id: "mastodon".into(),
        message: "Longer post with more detail for Mastodon...".into(),
        images: None,
    },
]).await;
```

---

## Supported Platforms (16)

| Platform | Strategy | Auth | Images | Max Length |
|----------|----------|------|--------|------------|
| Twitter/X | `TwitterStrategy` | OAuth2 bearer | Yes (upload) | 280 (URLs=23) |
| Bluesky | `BlueskyStrategy` | App password | Yes (blob) | 300 (URLs=27) |
| Mastodon | `MastodonStrategy` | OAuth2 bearer | Yes (media) | 500 |
| LinkedIn | `LinkedInStrategy` | OAuth2 bearer | Yes (3-step) | 3,000 |
| Facebook | `FacebookStrategy` | OAuth2 bearer | Yes (multi) | 63,206 |
| Instagram | `InstagramStrategy` | OAuth2 bearer | Yes | 2,200 |
| Discord Bot | `DiscordStrategy` | Bot token | Yes (multipart) | 2,000 |
| Discord Webhook | `DiscordWebhookStrategy` | Webhook URL | Yes (multipart) | 2,000 |
| Telegram | `TelegramStrategy` | Bot API | Yes (sendPhoto) | 4,096 |
| Slack | `SlackStrategy` | Bot token | Yes (3-step) | 40,000 |
| Dev.to | `DevtoStrategy` | API key | Yes (base64 md) | Unlimited |
| Nostr | `NostrStrategy` | Private key | No | 280 |
| YouTube | `YouTubeStrategy` | OAuth2 bearer | No | 5,000 |
| TikTok | `TikTokStrategy` | OAuth2 bearer | No | 2,200 |
| Reddit | `RedditStrategy` | OAuth2 bearer | No | 40,000 |
| Twitch | `TwitchStrategy` | OAuth2 bearer | No | 500 |

---

## Environment Variables

Each strategy reads specific environment variables via `from_env()`:

| Platform | Variables |
|----------|-----------|
| Twitter | `TWITTER_ACCESS_TOKEN` (or `TWITTER_ACCESS_TOKEN_KEY`) |
| Bluesky | `BLUESKY_IDENTIFIER`, `BLUESKY_PASSWORD`, `BLUESKY_HOST` (optional) |
| Mastodon | `MASTODON_ACCESS_TOKEN`, `MASTODON_HOST` (optional, defaults to mastodon.social) |
| LinkedIn | `LINKEDIN_ACCESS_TOKEN` |
| Facebook | `FACEBOOK_ACCESS_TOKEN`, `FACEBOOK_PAGE_ID` (optional) |
| Instagram | `INSTAGRAM_ACCESS_TOKEN` |
| Discord Bot | `DISCORD_BOT_TOKEN`, `DISCORD_CHANNEL_ID` |
| Discord Webhook | `DISCORD_WEBHOOK_URL` |
| Telegram | `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` |
| Slack | `SLACK_TOKEN`, `SLACK_CHANNEL` (optional, defaults to #general) |
| Dev.to | `DEVTO_API_KEY` |
| Nostr | `NOSTR_PRIVATE_KEY`, `NOSTR_RELAYS` (comma-separated) |
| YouTube | `YOUTUBE_ACCESS_TOKEN` |
| TikTok | `TIKTOK_ACCESS_TOKEN` |
| Reddit | `REDDIT_ACCESS_TOKEN`, `REDDIT_SUBREDDIT` (optional) |
| Twitch | `TWITCH_ACCESS_TOKEN`, `TWITCH_CLIENT_ID` |

---

## Image Support

Attach images using `PostOptions`:

```rust
use crosspost::{PostOptions, ImageEmbed};

let options = PostOptions {
    images: vec![ImageEmbed {
        data: std::fs::read("photo.jpg")?,
        alt: Some("A photo".into()),
        mime_type: Some("image/jpeg".into()),
    }],
};

let results = client.post("Check out this photo!", Some(&options)).await;
```

- Max 4 images per post
- MIME type auto-detected if not provided
- Supported: JPEG, PNG, GIF
- Image compression utilities available in `crosspost::util::images`

---

## Architecture

The library uses the **Strategy pattern**. Each platform is a `Strategy` implementation with credentials baked in at construction time. The `Client` orchestrates posting to multiple strategies concurrently.

```
Client::post("message")
    ├── TwitterStrategy::post()   → PostResult::Success / Failure
    ├── BlueskyStrategy::post()   → PostResult::Success / Failure
    └── MastodonStrategy::post()  → PostResult::Success / Failure
```

One strategy failing does not affect others. Results are collected as `Vec<PostResult>`.

### Server Feature

An optional SaaS server layer is available behind the `server` feature flag:

```toml
[dependencies]
crosspost = { git = "https://github.com/GraftAI-com/crosspost-rs.git", features = ["server"] }
```

This includes:
- Axum HTTP API with JWT authentication
- SurrealDB for data persistence
- OAuth2 flow handling for platform connections
- Rate limiting (global and per-user)
- Background scheduling for deferred posts
- Token refresh management

Run the server:
```bash
cargo run --features server --bin crosspost-server
```

---

## Development

```bash
cargo check                                          # Check library
cargo test                                           # Library tests (38)
cargo test --features server                         # All tests (61)
cargo clippy --all-targets -- -D warnings            # Lint library
cargo clippy --all-targets --features server -- -D warnings  # Lint everything
cargo fmt --all -- --check                           # Format check
```

---

## License

This project is licensed under the [Mozilla Public License 2.0](https://www.mozilla.org/MPL/2.0/). See [LICENSE](LICENSE).

## Acknowledgments

- Original [crosspost library](https://github.com/humanwhocodes/crosspost) by [Nicholas C. Zakas](https://humanwhocodes.com)
