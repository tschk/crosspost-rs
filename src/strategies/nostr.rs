use crate::env::{optional_env, required_env};
use crate::error::{Error, Result};
use crate::strategy::{PostResponse, Strategy};
use crate::types::{NostrCredentials, PostOptions};
use futures_util::{SinkExt, StreamExt};
use secp256k1::{Keypair, Message, Secp256k1, SecretKey, XOnlyPublicKey};
use sha2::{Digest, Sha256};
use tokio_tungstenite::tungstenite;

/// Strategy for publishing to the Nostr protocol.
///
/// Signs events with secp256k1 and publishes to one or more relays via WebSocket.
/// Supports hex and bech32 (nsec1) private key formats.
#[derive(Debug)]
pub struct NostrStrategy {
    secret_key: SecretKey,
    relays: Vec<String>,
}

impl NostrStrategy {
    pub fn new(credentials: NostrCredentials) -> Result<Self> {
        if credentials.private_key.is_empty() {
            return Err(Error::Validation(
                "Nostr private_key is required".to_string(),
            ));
        }
        if credentials.relays.is_empty() {
            return Err(Error::Validation(
                "Nostr requires at least one relay".to_string(),
            ));
        }
        let secret_key = Self::parse_private_key(&credentials.private_key)?;
        Ok(Self {
            secret_key,
            relays: credentials.relays,
        })
    }

    pub fn from_env() -> Result<Self> {
        let relays_str = optional_env("NOSTR_RELAYS").unwrap_or_default();
        let relays: Vec<String> = relays_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Self::new(NostrCredentials {
            private_key: required_env("NOSTR_PRIVATE_KEY")?,
            relays,
        })
    }

    fn parse_private_key(key_str: &str) -> Result<SecretKey> {
        if key_str.starts_with("nsec1") {
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
            let bytes = hex::decode(key_str)
                .map_err(|e| Error::Platform(format!("Invalid hex private key: {}", e)))?;
            SecretKey::from_slice(&bytes)
                .map_err(|e| Error::Platform(format!("Invalid private key: {}", e)))
        }
    }

    fn create_signed_event(secret_key: &SecretKey, content: &str) -> Result<serde_json::Value> {
        let secp = Secp256k1::new();
        let keypair = Keypair::from_secret_key(&secp, secret_key);
        let (pubkey, _) = XOnlyPublicKey::from_keypair(&keypair);
        let pubkey_hex = hex::encode(pubkey.serialize());

        let created_at = chrono::Utc::now().timestamp();
        let kind: u32 = 1;
        let tags: Vec<Vec<String>> = vec![];

        let serialized = serde_json::json!([0, pubkey_hex, created_at, kind, tags, content]);
        let serialized_str = serde_json::to_string(&serialized)
            .map_err(|e| Error::Platform(format!("Failed to serialize event: {}", e)))?;

        let mut hasher = Sha256::new();
        hasher.update(serialized_str.as_bytes());
        let id_bytes = hasher.finalize();
        let id_hex = hex::encode(id_bytes);

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
impl Strategy for NostrStrategy {
    fn name(&self) -> &str {
        "Nostr"
    }

    fn id(&self) -> &str {
        "nostr"
    }

    fn max_message_length(&self) -> usize {
        280
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        if let Some(opts) = options {
            if !opts.images.is_empty() {
                return Err(Error::Platform(
                    "Nostr does not support image uploads".to_string(),
                ));
            }
        }

        let event = Self::create_signed_event(&self.secret_key, message)?;

        let event_id = event["id"]
            .as_str()
            .ok_or_else(|| Error::Platform("Missing event ID".to_string()))?
            .to_string();

        let mut errors = Vec::new();
        let mut published = false;

        for relay in &self.relays {
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
            id: event_id,
            url: Some(url),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        // If we were able to construct the strategy, the key is valid
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(NostrStrategy::new(NostrCredentials {
            private_key: String::new(),
            relays: vec!["wss://relay.damus.io".to_string()],
        })
        .is_err());

        assert!(NostrStrategy::new(NostrCredentials {
            private_key: "abc".to_string(),
            relays: vec![],
        })
        .is_err());
    }

    #[test]
    fn test_rejects_empty_private_key() {
        let result = NostrStrategy::new(NostrCredentials {
            private_key: "".to_string(),
            relays: vec!["wss://relay.damus.io".to_string()],
        });
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("private_key is required"),
            "Expected private_key validation error, got: {}",
            err
        );
    }

    #[test]
    fn test_rejects_empty_relays() {
        let result = NostrStrategy::new(NostrCredentials {
            private_key: "some_hex_key".to_string(),
            relays: vec![],
        });
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("at least one relay"),
            "Expected relay validation error, got: {}",
            err
        );
    }

    #[test]
    fn test_event_creation() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut secp256k1::rand::thread_rng());

        let event = NostrStrategy::create_signed_event(&secret_key, "Hello, Nostr!").unwrap();
        assert_eq!(event["kind"], 1);
        assert_eq!(event["content"], "Hello, Nostr!");
        assert!(event["id"].as_str().unwrap().len() == 64);
        assert!(event["sig"].as_str().unwrap().len() == 128);
    }

    #[test]
    fn test_event_id_to_note() {
        let hex_id = "a".repeat(64);
        let note = NostrStrategy::event_id_to_note(&hex_id).unwrap();
        assert!(note.starts_with("note1"));
    }

    #[test]
    fn test_strategy_metadata() {
        let secp = Secp256k1::new();
        let (secret_key, _) = secp.generate_keypair(&mut secp256k1::rand::thread_rng());
        let key_hex = hex::encode(secret_key.secret_bytes());

        let s = NostrStrategy::new(NostrCredentials {
            private_key: key_hex,
            relays: vec!["wss://relay.damus.io".to_string()],
        })
        .unwrap();
        assert_eq!(s.id(), "nostr");
        assert_eq!(s.name(), "Nostr");
        assert_eq!(s.max_message_length(), 280);
    }
}
