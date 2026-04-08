# Security Audit Report

**Date:** 2026-02-16
**Target:** `crosspost-rs` workspace
**Auditor:** Gemini CLI Agent

---

## 1. Summary

The codebase generally follows Rust security best practices. Dependencies are standard and well-maintained. Input validation is present for critical vectors (request body size, image count, message length). No hardcoded secrets were found.

**Status:** ✅ **PASS** (Low Risk)

---

## 2. Findings

### 2.1 Dependencies
- **Standard Stack:** Uses industry-standard crates (`tokio`, `axum`, `serde`, `reqwest`, `thiserror`).
- **FFI Usage:**
  - `mozjpeg` (via `mozjpeg-sys`) and `oxipng` used for image optimization.
  - `secp256k1` (via `secp256k1-sys`) used for Nostr cryptography.
  - `surrealdb` (via `librocksdb-sys`) used for storage.
  - *Risk:* FFI introduces memory safety risks inherent to C, but these are widely used, battle-tested crates.
- **Supply Chain:** No obviously malicious or deprecated direct dependencies identified.

### 2.2 Unsafe Code
- **Internal Code:** No `unsafe` blocks were found in the `crosspost` crate's source code.
- **Dependencies:** `unsafe` is used in transitive dependencies (compression, crypto, database drivers), which is unavoidable and standard for this domain.

### 2.3 Panic Safety
- **Unwrap Usage:** `unwrap()` calls found in `crates/crosspost/src/strategies/*.rs` were verified to be contained strictly within `#[cfg(test)] mod tests` blocks.
- **Server Startup:** `crates/api/src/lib.rs` uses `expect()` for setting up signal handlers (Ctrl+C). This is acceptable as failure here is fatal for server startup.
- **Error Handling:** Production code uses `Result<T, Error>` and proper error propagation (`?`) instead of panicking.

### 2.4 Secret Handling
- **No Hardcoded Secrets:** Regex scans for patterns like `Bearer ...`, `API_KEY = ...` returned no results in source files.
- **Configuration:** Secrets are loaded via environment variables (`dotenvy` / `std::env`).
- **JWT:** The API server (`crates/api/src/lib.rs`) explicitly enforces a minimum length of 32 characters for `AUTH__JWT_SECRET` at startup.

### 2.5 Input Validation & Limits
- **HTTP Body Limit:** `Axum` server is configured with a **10MB** default body limit (`crates/api/src/lib.rs`), protecting against large payload DoS attacks.
- **Image Limits:** `crosspost::util::images::validate_images` enforces a strict maximum of **4 images** per post.
- **Message Length:** Each strategy implements `max_message_length()` and checks it before attempting to post.
- **CORS:** CORS is configured in `crates/api/src/lib.rs` to be restrictive by default unless `SERVER__CORS_ORIGINS` is set.
- **Security Headers:** The server applies standard security headers:
  - `X-Content-Type-Options: nosniff`
  - `X-Frame-Options: DENY`
  - `X-XSS-Protection: 1; mode=block`
  - `Strict-Transport-Security: max-age=31536000; includeSubDomains`

### 2.6 Database & Auth
- **Password Hashing:** Uses `argon2`, a modern, memory-hard hashing algorithm.
- **Database:** Uses `SurrealDB` (embedded RocksDB mode for this setup).

---

## 3. Recommendations

1.  **Dependency Auditing:** Integrate `cargo-audit` into the CI pipeline to automatically check for vulnerabilities in the dependency tree.
2.  **Rate Limiting:** Ensure the `governor`-based rate limiting (mentioned in `Cargo.toml`) is correctly applied to public-facing routes in `crates/api`.
3.  **Image Bomb Protection:** While `image` crate limits memory, ensure that very large dimensions (pixels) are rejected early in `image_dimensions()` before attempting full decompression/resize operations if not already handled by `DefaultBodyLimit`.
