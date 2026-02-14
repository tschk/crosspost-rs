# Crosspost Rust - Multi-Tenant Social Media Posting Platform

A complete rewrite of the JavaScript crosspost library into Rust, designed as a multi-tenant SaaS platform for marketing agencies.

## Overview

Crosspost Rust is a comprehensive social media management platform that allows marketing agencies to manage multiple clients (tenants), with each client having multiple team members who can connect and post to multiple social media accounts per platform.

## Features

- **Multi-Tenant Architecture**: Complete isolation between different marketing agency clients
- **OAuth2 Integration**: Support for 10+ social media platforms with automatic token refresh
- **Multiple Accounts per Platform**: Users can connect multiple accounts from the same platform (e.g., 3 different Twitter accounts)
- **Crossposting**: Post to multiple platforms and accounts simultaneously
- **Scheduled Posts**: Schedule content for future publishing
- **Rate Limiting**: Per-platform rate limit tracking
- **Token Management**: Automatic token refresh before expiry
- **Secure Storage**: Encrypted token storage in SurrealDB

## Supported Platforms

- Twitter/X (OAuth 2.0)
- Facebook (Meta Graph API OAuth 2.0)
- Instagram (Meta Graph API OAuth 2.0)
- LinkedIn (OAuth 2.0)
- YouTube (Google OAuth 2.0)
- TikTok (OAuth 2.0)
- Reddit (OAuth 2.0)
- Twitch (OAuth 2.0)
- Slack (OAuth 2.0)
- Telegram (Bot API)

## Tech Stack

- **Web Framework**: Axum
- **Primary Database**: SurrealDB (for user data, tenant info, OAuth tokens, post history)
- **Cache Layer**: RocksDB (for fast token lookups, rate limit tracking)
- **OAuth**: oauth2 crate
- **Async Runtime**: Tokio

## Project Structure

```
crosspost-rs/
├── Cargo.toml              # Workspace configuration
├── crates/
│   ├── core/               # Shared types, errors, config
│   ├── auth/               # OAuth2 handlers, token management
│   ├── db/                 # SurrealDB + RocksDB integration
│   ├── platforms/          # Platform-specific API clients
│   └── api/                # Axum routes, middleware, handlers
├── migrations/             # Database migrations
├── Dockerfile              # Docker build configuration
├── docker-compose.yml      # Docker Compose setup
└── README.md
```

## Getting Started

### Prerequisites

- Rust 1.83 or higher
- Docker and Docker Compose (optional, for containerized deployment)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/GraftAI-com/crosspost-rs.git
cd crosspost-rs
```

2. Copy the example environment file:
```bash
cp .env.example .env
```

3. Edit `.env` with your OAuth credentials for the platforms you want to support.

### Running Locally

Build and run the server:

```bash
cargo build --release
cargo run --bin crosspost-server
```

The server will start on `http://localhost:3000` by default.

### Running with Docker

Build and start the services:

```bash
docker-compose up -d
```

This will start:
- The Crosspost API server on port 3000
- SurrealDB on port 8000

## API Endpoints

### Authentication & OAuth

- `POST /auth/{platform}/connect` - Initiate OAuth flow for a platform
- `GET /auth/{platform}/callback` - OAuth callback handler
- `DELETE /auth/accounts/{account_id}` - Disconnect a connected account

### Account Management

- `GET /accounts` - List all connected accounts for the authenticated user

### Posting

- `POST /post` - Create and post content to multiple platforms/accounts
- `GET /posts` - Get post history for the authenticated user
- `POST /schedule` - Schedule a post for future publishing

### Health

- `GET /health` - Health check endpoint

## API Usage Examples

### 1. Initiate OAuth Connection

```bash
curl -X POST http://localhost:3000/auth/twitter/connect
```

Response:
```json
{
  "authorization_url": "https://twitter.com/i/oauth2/authorize?...",
  "state": "random-state-token"
}
```

### 2. List Connected Accounts

```bash
curl http://localhost:3000/accounts
```

### 3. Create a Post

```bash
curl -X POST http://localhost:3000/post \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Hello from Crosspost Rust!",
    "account_ids": ["account-uuid-1", "account-uuid-2"]
  }'
```

### 4. Schedule a Post

```bash
curl -X POST http://localhost:3000/schedule \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Scheduled post content",
    "account_ids": ["account-uuid-1"],
    "scheduled_for": "2026-02-15T12:00:00Z"
  }'
```

## Configuration

Configuration is managed through environment variables. All variables use the double underscore (`__`) separator for nested configuration.

### Server Configuration

- `SERVER__HOST`: Server host (default: `127.0.0.1`)
- `SERVER__PORT`: Server port (default: `3000`)
- `SERVER__BASE_URL`: Base URL for callbacks (e.g., `http://localhost:3000`)

### Database Configuration

- `DATABASE__SURREALDB_URL`: SurrealDB connection URL (default: `memory://`)
- `DATABASE__ROCKSDB_PATH`: Path to RocksDB data directory (default: `./data/rocksdb`)

### OAuth Configuration

For each platform, configure:
- `OAUTH__{PLATFORM}__CLIENT_ID`: OAuth client ID
- `OAUTH__{PLATFORM}__CLIENT_SECRET`: OAuth client secret
- `OAUTH__{PLATFORM}__REDIRECT_URI`: OAuth redirect URI

Replace `{PLATFORM}` with: `TWITTER`, `FACEBOOK`, `INSTAGRAM`, `LINKEDIN`, `YOUTUBE`, `TIKTOK`, `REDDIT`, `TWITCH`, `SLACK`, or `TELEGRAM`.

## Architecture

### Multi-Tenant Design

Each tenant (marketing agency client) has complete data isolation:
- Tenant-scoped database queries
- Separate OAuth tokens per tenant
- Independent rate limiting per tenant

### OAuth Flow

1. Client initiates OAuth by calling `/auth/{platform}/connect`
2. Server returns authorization URL
3. User authorizes on the platform
4. Platform redirects to `/auth/{platform}/callback`
5. Server exchanges code for access token
6. Token is stored securely in SurrealDB
7. Automatic refresh before expiry

### Token Management

- Access tokens stored encrypted in SurrealDB
- Refresh tokens used for automatic renewal
- Token expiry tracked and monitored
- Automatic refresh 5 minutes before expiry

### Rate Limiting

- Per-platform rate limits tracked in RocksDB
- Prevents exceeding platform API limits
- Configurable limits per platform

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

## Deployment

### Docker Deployment

The project includes a multi-stage Dockerfile for optimized production builds:

```bash
docker build -t crosspost-rs .
docker run -p 3000:3000 --env-file .env crosspost-rs
```

### Using Docker Compose

For a complete setup with SurrealDB:

```bash
docker-compose up -d
```

## Security Considerations

- All OAuth tokens are stored securely in SurrealDB
- Environment variables used for sensitive configuration
- HTTPS recommended for production deployments
- Tenant isolation enforced at the middleware layer
- Input validation on all endpoints

## Roadmap

- [ ] Complete implementation of all 10 platforms
- [ ] Add comprehensive test coverage
- [ ] Implement JWT-based authentication for API users
- [ ] Add webhook support for platform events
- [ ] Implement analytics and reporting
- [ ] Add media upload support for all platforms
- [ ] Implement scheduled post processing worker
- [ ] Add GraphQL API option
- [ ] Implement API rate limiting
- [ ] Add observability (metrics, distributed tracing)

## License

Apache License 2.0 - See LICENSE file for details

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Support

For issues, questions, or contributions, please open an issue on GitHub.
