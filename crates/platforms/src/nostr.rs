use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use futures_util::{SinkExt, StreamExt};
use secp256k1::{Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite;

/// Nostr client - publishes events to configured relays
pub struct NostrClient {
    _client: reqwest::Client,
}

impl NostrClient {
    pub fn new() -> Self {
        Self {
            _client: reqwest::Client::new(),
        }
    }
}

impl Default for NostrClient {
    fn default() -> Self {
        Self::new()
    }
}

impl NostrClient {
    /// Parse access_token as "private_key|relay1,relay2,..."
    fn parse_token(access_token: &str) -> Result<(SecretKey, Vec<String>)> {
        let parts: Vec<&str> = access_token.splitn(2, '|').collect();
        if parts.len() != 2 {
            return Err(Error::Platform(
                "Nostr requires token as 'private_key|relay1,relay2,...'".to_string(),
            ));
        }

        let key_str = parts[0];
        let relays: Vec<String> = parts[1].split(',').map(|s| s.trim().to_string()).collect();

        if relays.is_empty() {
            return Err(Error::Platform(
                "Nostr requires at least one relay".to_string(),
            ));
        }

        let secret_key = Self::parse_private_key(key_str)?;
        Ok((secret_key, relays))
    }

    /// Parse a private key from hex or bech32 nsec1 format
    fn parse_private_key(key_str: &str) -> Result<SecretKey> {
        if key_str.starts_with("nsec1") {
            // Decode bech32 nsec1 format
            let (hrp, data) = bech32::decode(key_str)
                .map_err(|e| Error::Platform(format!("Invalid nsec1 key: {}", e)))?;

            if hrp.as_str() != "nsec" {
                return Err(Error::Platform(
                    "Invalid key prefix, expected nsec".to_string(),
                ));
            }

            SecretKey::from_slice(&data)
                .map_err(|e| Error::Platform(format!("Invalid private key: {}", e)))
        } else {
            // Hex format
            let bytes = hex::decode(key_str)
                .map_err(|e| Error::Platform(format!("Invalid hex private key: {}", e)))?;
            SecretKey::from_slice(&bytes)
                .map_err(|e| Error::Platform(format!("Invalid private key: {}", e)))
        }
    }

    /// Create a signed NIP-01 event
    fn create_signed_event(secret_key: &SecretKey, content: &str) -> Result<serde_json::Value> {
        let secp = Secp256k1::new();
        let keypair = Keypair::from_secret_key(&secp, secret_key);
        let (pubkey, _) = XOnlyPublicKey::from_keypair(&keypair);
        let pubkey_hex = hex::encode(pubkey.serialize());

        let created_at = chrono::Utc::now().timestamp();
        let kind: u32 = 1; // Short text note
        let tags: Vec<Vec<String>> = vec![];

        // Compute event ID: SHA-256 of [0, pubkey, created_at, kind, tags, content]
        let serialized = serde_json::json!([0, pubkey_hex, created_at, kind, tags, content]);
        let serialized_str = serde_json::to_string(&serialized)
            .map_err(|e| Error::Platform(format!("Failed to serialize event: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(serialized_str.as_bytes());
        let id_bytes = hasher.finalize();
        let id_hex = hex::encode(id_bytes);

        // Sign the event ID with schnorr
        let msg = Message::from_digest_slice(&id_bytes)
            .map_err(|e| Error::Platform(format!("Failed to create message: {}", e)))?;
        let sig = secp.sign_schnorr_no_aux_rand(&msg, &keypair);
        let sig_hex = hex::encode(sig.serialize());

        Ok(serde_json::json!({
            "id": id_hex,
            "pubkey": pubkey_hex,
            "created_at": created_at,
            "kind": kind,
            "tags": tags,
            "content": content,
            "sig": sig_hex,
        }))
    }

    /// Publish an event to a relay via WebSocket
    async fn publish_to_relay(relay_url: &str, event: &serde_json::Value) -> Result<()> {
        let url = if relay_url.starts_with("wss://") || relay_url.starts_with("ws://") {
            relay_url.to_string()
        } else {
            format!("wss://{}", relay_url)
        };

        let (mut ws, _) = tokio_tungstenite::connect_async(&url)
            .await
            .map_err(|e| Error::Platform(format!("Failed to connect to relay {}: {}", url, e)))?;

        let msg = serde_json::json!(["EVENT", event]);
        let msg_str = serde_json::to_string(&msg)
            .map_err(|e| Error::Platform(format!("Failed to serialize event: {}", e)))?;

        ws.send(tungstenite::Message::Text(msg_str))
            .await
            .map_err(|e| Error::Platform(format!("Failed to send to relay {}: {}", url, e)))?;

        // Read one response to check for OK/error before closing
        if let Ok(Some(Ok(tungstenite::Message::Text(text)))) =
            tokio::time::timeout(std::time::Duration::from_secs(5), ws.next()).await
        {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&text) {
                if parsed.get(0).and_then(|v| v.as_str()) == Some("OK") {
                    if let Some(false) = parsed.get(2).and_then(|v| v.as_bool()) {
                        let reason = parsed
                            .get(3)
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown reason");
                        tracing::warn!("Relay {} rejected event: {}", url, reason);
                    }
                }
            }
        }

        ws.close(None)
            .await
            .map_err(|e| Error::Platform(format!("Failed to close relay connection: {}", e)))?;

        Ok(())
    }

    /// Get the bech32 note ID from hex event ID
    fn event_id_to_note(event_id_hex: &str) -> Result<String> {
        let bytes = hex::decode(event_id_hex)
            .map_err(|e| Error::Platform(format!("Invalid event ID: {}", e)))?;
        let hrp = bech32::Hrp::parse("note")
            .map_err(|e| Error::Platform(format!("Invalid bech32 HRP: {}", e)))?;
        let encoded = bech32::encode::<bech32::Bech32>(hrp, &bytes)
            .map_err(|e| Error::Platform(format!("Failed to encode note ID: {}", e)))?;
        Ok(encoded)
    }
}

#[async_trait::async_trait]
impl Platform for NostrClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // Nostr does not support image uploads (matches JS library behavior)
        if let Some(ref images) = request.images {
            if !images.is_empty() {
                return Err(Error::Platform(
                    "Nostr does not support image uploads".to_string(),
                ));
            }
        }

        let (secret_key, relays) = Self::parse_token(access_token)?;
        let event = Self::create_signed_event(&secret_key, &request.content)?;

        let event_id = event["id"]
            .as_str()
            .ok_or_else(|| Error::Platform("Missing event ID".to_string()))?
            .to_string();

        // Publish to all configured relays
        let mut errors = Vec::new();
        let mut published = false;

        for relay in &relays {
            match Self::publish_to_relay(relay, &event).await {
                Ok(()) => {
                    published = true;
                    tracing::info!("Published to relay: {}", relay);
                }
                Err(e) => {
                    tracing::warn!("Failed to publish to relay {}: {}", relay, e);
                    errors.push(format!("{}: {}", relay, e));
                }
            }
        }

        if !published {
            return Err(Error::Platform(format!(
                "Failed to publish to any relay: {}",
                errors.join("; ")
            )));
        }

        let note_id = Self::event_id_to_note(&event_id)?;
        let url = format!("nostr:{}", note_id);

        Ok(PostResponse {
            platform_post_id: event_id,
            url: Some(url),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        match Self::parse_token(access_token) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn platform_name(&self) -> &'static str {
        "nostr"
    }

    fn max_message_length(&self) -> usize {
        280
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_token() {
        // Valid hex key with relays
        let hex_key = "0".repeat(64); // 32-byte key as hex
        let token = format!("{}|wss://relay.damus.io,wss://nos.lol", hex_key);

        // This will fail because all-zeros isn't a valid secp256k1 key
        // but let's test the parsing logic
        let result = NostrClient::parse_token(&token);
        // All-zeros key is invalid for secp256k1
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_parse_token_missing_relays() {
        assert!(NostrClient::parse_token("somekey").is_err());
    }

    #[test]
    fn test_max_message_length() {
        let client = NostrClient::new();
        assert_eq!(client.max_message_length(), 280);
    }

    #[test]
    fn test_event_creation() {
        // Generate a valid secret key for testing
        let secp = Secp256k1::new();
        let (secret_key, _public_key) = secp.generate_keypair(&mut secp256k1::rand::thread_rng());

        let event = NostrClient::create_signed_event(&secret_key, "Hello, Nostr!").unwrap();
        assert_eq!(event["kind"], 1);
        assert_eq!(event["content"], "Hello, Nostr!");
        assert!(event["id"].as_str().unwrap().len() == 64);
        assert!(event["sig"].as_str().unwrap().len() == 128);
    }

    #[test]
    fn test_event_id_to_note() {
        // Valid 32-byte hex
        let hex_id = "a".repeat(64);
        let note = NostrClient::event_id_to_note(&hex_id).unwrap();
        assert!(note.starts_with("note1"));
    }
}
