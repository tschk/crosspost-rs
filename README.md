# crosspost-rs

🚀 **Multi-tenant SaaS platform for cross-posting content to social media** - Rust rewrite of the popular [crosspost library](https://github.com/humanwhocodes/crosspost)

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.83%2B-orange.svg)](https://www.rust-lang.org)
[![Status](https://img.shields.io/badge/status-alpha-yellow.svg)]()

---

## ⚠️ Project Status: Alpha (Foundation Complete)

This is an **active rewrite** from JavaScript to Rust. The foundation is complete (~20% of planned features), but significant work remains. See [TODO.md](TODO.md) for the complete task list and progress.

**Current Capabilities:**
- ✅ Core types and error handling
- ✅ Database layer (SurrealDB + RocksDB)
- ✅ Basic OAuth structure
- ✅ API framework with Axum
- ✅ Docker deployment setup
- 🚧 Platform integrations (partial)
- 📋 Rate limiting (planned)
- 📋 Scheduling (planned)
- 📋 Production testing (planned)

**See [TODO.md](TODO.md) for detailed implementation status and roadmap.**

---

## 🎯 Overview

Crosspost-RS is a complete rewrite and expansion of the original JavaScript crosspost library, transforming it from a simple utility into a production-ready multi-tenant SaaS platform. Built for marketing agencies managing multiple clients, each with multiple social media accounts.

### Key Differences from JavaScript Version

| Feature | JavaScript Library | Rust SaaS Platform |
|---------|-------------------|-------------------|
| Architecture | Single-user library | Multi-tenant SaaS |
| Auth | API keys/tokens | OAuth2 flows |
| Storage | None | SurrealDB + RocksDB |
| Accounts | One per platform | Multiple per platform per user |
| Deployment | NPM package | Docker containers |
| API | Client library | REST API |
| Scheduling | None | Built-in scheduler |
| Rate Limiting | Manual | Automatic per-platform |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     Axum API Server                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐ │
│  │   OAuth      │  │   Posting    │  │  Scheduling  │ │
│  │  Endpoints   │  │  Endpoints   │  │  Endpoints   │ │
│  └──────────────┘  └──────────────┘  └──────────────┘ │
└─────────────────────────────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
  ┌─────▼─────┐      ┌──────▼──────┐     ┌─────▼─────┐
  │ SurrealDB │      │  RocksDB    │     │ Platform  │
  │           │      │             │     │  Clients  │
  │ • Users   │      │ • Tokens    │     │           │
  │ • Tenants │      │ • Rate      │     │ Twitter   │
  │ • Accounts│      │   Limits    │     │ Facebook  │
  │ • Posts   │      │             │     │ Instagram │
  └───────────┘      └─────────────┘     │ LinkedIn  │
                                          │ YouTube   │
                                          │ TikTok    │
                                          │ Reddit    │
                                          │ Twitch    │
                                          │ Slack     │
                                          │ Telegram  │
                                          └───────────┘
```

### Crate Structure

```
crosspost-rs/
├── crates/
│   ├── core/           # Shared types, errors, config (267 lines)
│   ├── auth/           # OAuth2, token management (250 lines)
│   ├── db/             # SurrealDB + RocksDB clients (370 lines)
│   ├── platforms/      # Platform API clients (290 lines)
│   └── api/            # Axum server & endpoints (560 lines)
├── migrations/         # Database migrations (planned)
├── Dockerfile          # Production container
├── docker-compose.yml  # Local development
└── TODO.md            # Complete task list
```

---

## 🚀 Quick Start

### Prerequisites

- Rust 1.83 or later
- Docker & Docker Compose (for easy setup)
- Or: SurrealDB and RocksDB locally

### Option 1: Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/GraftAI-com/crosspost-rs.git
cd crosspost-rs

# Copy environment template
cp .env.example .env

# Edit .env with your OAuth credentials
# (See "Platform Setup" section below)

# Start all services
docker-compose up -d

# Check logs
docker-compose logs -f crosspost-api
```

API will be available at `http://localhost:3000`

### Option 2: Local Development

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install SurrealDB
curl -sSf https://install.surrealdb.com | sh

# Clone and build
git clone https://github.com/GraftAI-com/crosspost-rs.git
cd crosspost-rs
cargo build --release

# Start SurrealDB
surreal start --log trace --user root --pass root memory

# Set environment variables
export DATABASE__SURREALDB_URL=ws://localhost:8000
export DATABASE__ROCKSDB_PATH=./data/rocksdb
export SERVER__HOST=0.0.0.0
export SERVER__PORT=3000

# Run the server
cargo run --bin crosspost-server
```

---

## 📚 API Documentation

### Health Check

```bash
curl http://localhost:3000/health
```

### OAuth Flow

1. **Initiate Connection**
```bash
curl -X POST http://localhost:3000/auth/twitter/connect \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Tenant-ID: YOUR_TENANT_ID"

# Returns:
{
  "authorization_url": "https://twitter.com/oauth/authorize?...",
  "state": "random-csrf-token"
}
```

2. **User authorizes on platform** (redirected to authorization_url)

3. **Callback handled automatically** at `/auth/{platform}/callback`

4. **Account is connected** and stored in database

### List Connected Accounts

```bash
curl http://localhost:3000/accounts \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Tenant-ID: YOUR_TENANT_ID"
```

### Create Cross-Post

```bash
curl -X POST http://localhost:3000/post \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Tenant-ID: YOUR_TENANT_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Hello from Crosspost-RS! 🚀",
    "platform_accounts": [
      "twitter-account-uuid",
      "facebook-account-uuid"
    ],
    "media": []
  }'

# Returns:
{
  "post_id": "uuid",
  "results": [
    {
      "platform": "twitter",
      "success": true,
      "platform_post_id": "1234567890",
      "url": "https://twitter.com/user/status/1234567890"
    },
    {
      "platform": "facebook",
      "success": true,
      "platform_post_id": "98765_43210",
      "url": "https://facebook.com/98765/posts/43210"
    }
  ]
}
```

### Schedule a Post

```bash
curl -X POST http://localhost:3000/schedule \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "X-Tenant-ID: YOUR_TENANT_ID" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Scheduled post for tomorrow",
    "platform_accounts": ["twitter-account-uuid"],
    "scheduled_for": "2026-02-15T09:00:00Z"
  }'
```

---

## 🔧 Platform Setup

Each platform requires OAuth2 credentials. Follow these guides:

### Twitter/X
1. Go to [Twitter Developer Portal](https://developer.twitter.com/en/portal/dashboard)
2. Create a new app
3. Enable OAuth 2.0
4. Set redirect URI: `http://localhost:3000/auth/twitter/callback`
5. Add to `.env`:
```env
TWITTER_CLIENT_ID=your_client_id
TWITTER_CLIENT_SECRET=your_client_secret
```

### Facebook
1. Go to [Facebook Developers](https://developers.facebook.com/)
2. Create an app
3. Add "Facebook Login" product
4. Set Valid OAuth Redirect URIs: `http://localhost:3000/auth/facebook/callback`
5. Add to `.env`:
```env
FACEBOOK_APP_ID=your_app_id
FACEBOOK_APP_SECRET=your_app_secret
```

### Instagram
(Uses Facebook OAuth - requires business account)
```env
INSTAGRAM_APP_ID=your_facebook_app_id
INSTAGRAM_APP_SECRET=your_facebook_app_secret
```

### LinkedIn
1. Go to [LinkedIn Developers](https://www.linkedin.com/developers/)
2. Create an app
3. Request "Share on LinkedIn" product
4. Add redirect URL: `http://localhost:3000/auth/linkedin/callback`
```env
LINKEDIN_CLIENT_ID=your_client_id
LINKEDIN_CLIENT_SECRET=your_client_secret
```

*See [SETUP.md](SETUP.md) for complete platform setup instructions*

---

## 🔐 Multi-Tenant Architecture

### Tenant Isolation

Every request must include:
- `Authorization: Bearer <jwt_token>` - Contains user ID and tenant ID
- `X-Tenant-ID: <tenant_id>` - Verified against JWT

### Database Scoping

All queries are automatically scoped by tenant ID:
```rust
// Automatic tenant scoping
let accounts = db.list_connected_accounts_by_user(user_id).await?;
// Only returns accounts for the user's tenant
```

### Security Features

- JWT-based authentication
- Tenant-level data isolation
- Per-user rate limiting
- OAuth token encryption at rest
- Audit logging (planned)

---

## 📊 Supported Platforms

| Platform | OAuth | Post Text | Post Images | Status |
|----------|-------|-----------|-------------|--------|
| Twitter  | ✅    | ✅        | 📋         | 🚧 In Progress |
| Facebook | ✅    | ✅        | 📋         | 🚧 In Progress |
| Instagram| ✅    | ✅        | 📋         | 🚧 In Progress |
| LinkedIn | 📋   | 📋        | 📋         | 📋 Planned |
| YouTube  | 📋   | 📋        | 📋         | 📋 Planned |
| TikTok   | 📋   | 📋        | 📋         | 📋 Planned |
| Reddit   | 📋   | 📋        | 📋         | 📋 Planned |
| Twitch   | 📋   | 📋        | 📋         | 📋 Planned |
| Slack    | 📋   | 📋        | 📋         | 📋 Planned |
| Telegram | 📋   | 📋        | 📋         | 📋 Planned |

Legend: ✅ Complete | 🚧 In Progress | 📋 Planned

---

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run specific crate tests
cargo test -p crosspost-core
cargo test -p crosspost-platforms

# Run with output
cargo test -- --nocapture

# Run integration tests
cargo test --test '*'
```

⚠️ **Note**: Test suite is currently being developed. See [TODO.md](TODO.md) for testing roadmap.

---

## 🚦 Rate Limiting

Each platform has different rate limits. Crosspost-RS automatically manages these:

| Platform | Limit | Window |
|----------|-------|--------|
| Twitter  | 300 posts | 3 hours |
| Facebook | Varies | Per page |
| Instagram| 25 posts | 24 hours |
| LinkedIn | 150 posts | 24 hours |

Rate limits are tracked in RocksDB with sliding window algorithm.

---

## 📈 Monitoring

### Health Endpoints

```bash
# Basic health check
curl http://localhost:3000/health

# Database health (planned)
curl http://localhost:3000/health/db

# Metrics (planned, Prometheus format)
curl http://localhost:3000/metrics
```

### Logging

Structured logging with tracing:

```bash
# Set log level
export RUST_LOG=debug

# Or per-module
export RUST_LOG=crosspost_api=debug,crosspost_platforms=trace
```

## 📄 License

Polyform Shield 1.0.0 - See [LICENSE](LICENSE) for details

---

## 🙏 Acknowledgments

- Original [crosspost library](https://github.com/humanwhocodes/crosspost) by [Nicholas C. Zakas](https://humanwhocodes.com)
- Inspired by the need for enterprise-grade social media management tools
- Built with amazing Rust ecosystem crates: Axum, SurrealDB, Tokio, and many more
