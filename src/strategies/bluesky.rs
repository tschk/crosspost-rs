use crate::env::{optional_env, required_env};
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{BlueskyCredentials, PostOptions};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tokio::sync::RwLock;

/// Strategy for posting to Bluesky via the AT Protocol.
///
/// Supports rich text facets (URLs, @mentions, #hashtags), image uploads with
/// aspect ratios, and configurable PDS host. URLs > 27 chars count as 27 for length.
/// Sessions are cached and reused for up to 5 minutes to avoid re-authenticating per post.
pub struct BlueskyStrategy {
    client: reqwest::Client,
    credentials: BlueskyCredentials,
    host: String,
    cached_session: RwLock<Option<CachedSession>>,
}

struct CachedSession {
    session: CreateSessionResponse,
    created_at: Instant,
}

/// Session cache duration (5 minutes). Bluesky JWTs last longer,
/// but re-authenticating every 5 minutes is a safe balance.
const SESSION_CACHE_DURATION: std::time::Duration = std::time::Duration::from_secs(300);

impl BlueskyStrategy {
    /// Create a new Bluesky strategy with the given credentials.
    pub fn new(credentials: BlueskyCredentials) -> Result<Self> {
        if credentials.identifier.is_empty() {
            return Err(Error::Validation(
                "Bluesky identifier is required".to_string(),
            ));
        }
        if credentials.password.is_empty() {
            return Err(Error::Validation(
                "Bluesky password is required".to_string(),
            ));
        }
        let host = credentials
            .host
            .clone()
            .unwrap_or_else(|| "bsky.social".to_string());
        crate::util::hosts::validate_public_host(&host)
            .map_err(|e| Error::Validation(format!("Invalid Bluesky host: {}", e)))?;
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            host,
            cached_session: RwLock::new(None),
        })
    }

    /// Create a Bluesky strategy from environment variables.
    ///
    /// Reads `BLUESKY_IDENTIFIER`, `BLUESKY_PASSWORD`, and optionally `BLUESKY_HOST`.
    pub fn from_env() -> Result<Self> {
        Self::new(BlueskyCredentials {
            identifier: required_env("BLUESKY_IDENTIFIER")?,
            password: required_env("BLUESKY_PASSWORD")?,
            host: optional_env("BLUESKY_HOST"),
        })
    }

    /// Get a cached session or create a new one if expired/missing.
    async fn get_or_create_session(&self) -> Result<CreateSessionResponse> {
        // Check cache first
        {
            let cache = self.cached_session.read().await;
            if let Some(ref cached) = *cache {
                if cached.created_at.elapsed() < SESSION_CACHE_DURATION {
                    return Ok(cached.session.clone());
                }
            }
        }

        // Cache miss or expired — create new session
        let session = self.create_session_fresh().await?;

        // Store in cache
        {
            let mut cache = self.cached_session.write().await;
            *cache = Some(CachedSession {
                session: session.clone(),
                created_at: Instant::now(),
            });
        }

        Ok(session)
    }

    async fn create_session_fresh(&self) -> Result<CreateSessionResponse> {
        let body = CreateSessionRequest {
            identifier: self.credentials.identifier.clone(),
            password: self.credentials.password.clone(),
        };

        let response = self
            .client
            .post(format!(
                "https://{}/xrpc/com.atproto.server.createSession",
                self.host
            ))
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Bluesky API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Bluesky login failed: {}",
                error_text
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Bluesky session: {}", e)))
    }

    async fn upload_blob(&self, jwt: &str, data: &[u8], mime_type: &str) -> Result<BlobRef> {
        let response = self
            .client
            .post(format!(
                "https://{}/xrpc/com.atproto.repo.uploadBlob",
                self.host
            ))
            .bearer_auth(jwt)
            .header("Content-Type", mime_type)
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Bluesky blob upload error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Bluesky blob upload failed: {}",
                error_text
            )));
        }

        let blob_response: UploadBlobResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse blob response: {}", e)))?;

        Ok(blob_response.blob)
    }

    fn detect_facets(text: &str) -> Vec<Facet> {
        let mut facets = Vec::new();

        // URL detection
        for (start, _) in text
            .match_indices("http://")
            .chain(text.match_indices("https://"))
        {
            let rest = &text[start..];
            let end = rest
                .find(|c: char| c.is_whitespace() || c == ')' || c == ']')
                .unwrap_or(rest.len());
            let url = &rest[..end];
            facets.push(Facet {
                index: FacetIndex {
                    byte_start: start,
                    byte_end: start + end,
                },
                features: vec![FacetFeature::Link {
                    uri: url.to_string(),
                }],
            });
        }

        // @mention detection
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'@' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
                let start = i;
                i += 1;
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric()
                        || bytes[i] == b'.'
                        || bytes[i] == b'-'
                        || bytes[i] == b'_')
                {
                    i += 1;
                }
                let handle = &text[start + 1..i];
                if handle.contains('.') && handle.len() > 2 {
                    facets.push(Facet {
                        index: FacetIndex {
                            byte_start: start,
                            byte_end: i,
                        },
                        features: vec![FacetFeature::Mention {
                            did: handle.to_string(),
                        }],
                    });
                }
            } else {
                i += 1;
            }
        }

        // #hashtag detection
        i = 0;
        while i < bytes.len() {
            if bytes[i] == b'#' && (i == 0 || bytes[i - 1].is_ascii_whitespace()) {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let tag = &text[start + 1..i];
                if !tag.is_empty() {
                    facets.push(Facet {
                        index: FacetIndex {
                            byte_start: start,
                            byte_end: i,
                        },
                        features: vec![FacetFeature::Tag {
                            tag: tag.to_string(),
                        }],
                    });
                }
            } else {
                i += 1;
            }
        }

        facets
    }
}

// --- Bluesky API types ---

#[derive(Serialize)]
struct CreateSessionRequest {
    identifier: String,
    password: String,
}

#[derive(Deserialize, Clone)]
struct CreateSessionResponse {
    #[serde(rename = "accessJwt")]
    access_jwt: String,
    did: String,
    handle: String,
}

#[derive(Serialize)]
struct CreateRecordRequest {
    repo: String,
    collection: String,
    record: BlueskyPost,
}

#[derive(Serialize)]
struct BlueskyPost {
    #[serde(rename = "$type")]
    record_type: String,
    text: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    facets: Option<Vec<Facet>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embed: Option<BlueskyEmbed>,
}

#[derive(Serialize)]
struct Facet {
    index: FacetIndex,
    features: Vec<FacetFeature>,
}

#[derive(Serialize)]
struct FacetIndex {
    #[serde(rename = "byteStart")]
    byte_start: usize,
    #[serde(rename = "byteEnd")]
    byte_end: usize,
}

#[derive(Serialize)]
#[serde(tag = "$type")]
enum FacetFeature {
    #[serde(rename = "app.bsky.richtext.facet#link")]
    Link { uri: String },
    #[serde(rename = "app.bsky.richtext.facet#mention")]
    Mention { did: String },
    #[serde(rename = "app.bsky.richtext.facet#tag")]
    Tag { tag: String },
}

#[derive(Serialize)]
#[serde(tag = "$type")]
enum BlueskyEmbed {
    #[serde(rename = "app.bsky.embed.images")]
    Images { images: Vec<BlueskyImage> },
}

#[derive(Serialize)]
struct BlueskyImage {
    alt: String,
    image: BlobRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "aspectRatio")]
    aspect_ratio: Option<AspectRatio>,
}

#[derive(Serialize, Deserialize, Clone)]
struct BlobRef {
    #[serde(rename = "$type")]
    blob_type: String,
    #[serde(rename = "ref")]
    blob_ref: BlobLink,
    #[serde(rename = "mimeType")]
    mime_type: String,
    size: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct BlobLink {
    #[serde(rename = "$link")]
    link: String,
}

#[derive(Serialize)]
struct AspectRatio {
    width: u32,
    height: u32,
}

#[derive(Deserialize)]
struct CreateRecordResponse {
    uri: String,
    cid: String,
}

#[derive(Deserialize)]
struct UploadBlobResponse {
    blob: BlobRef,
}

#[async_trait::async_trait]
impl Strategy for BlueskyStrategy {
    fn name(&self) -> &str {
        "Bluesky"
    }

    fn id(&self) -> &str {
        "bluesky"
    }

    fn max_message_length(&self) -> usize {
        300
    }

    fn calculate_message_length(&self, message: &str) -> usize {
        // Bluesky counts URLs > 27 chars as 27
        let mut length = 0;
        let mut remaining = message;

        while let Some(url_start) = remaining
            .find("http://")
            .or_else(|| remaining.find("https://"))
        {
            length += remaining[..url_start].len();
            let after_url = &remaining[url_start..];
            let url_end = after_url
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_url.len());
            length += url_end.min(27);
            remaining = &after_url[url_end..];
        }
        length += remaining.len();
        length
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let session = self.get_or_create_session().await?;

        let images = get_images(options);
        let embed = if !images.is_empty() {
            let mut bsky_images = Vec::new();
            for img in images.iter().take(4) {
                let mime = img
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "image/jpeg".to_string());
                let blob = self
                    .upload_blob(&session.access_jwt, &img.data, &mime)
                    .await?;

                let aspect_ratio =
                    crate::util::images::image_dimensions(&img.data)
                        .ok()
                        .map(|(w, h)| AspectRatio {
                            width: w,
                            height: h,
                        });

                bsky_images.push(BlueskyImage {
                    alt: img.alt.clone().unwrap_or_default(),
                    image: blob,
                    aspect_ratio,
                });
            }
            Some(BlueskyEmbed::Images {
                images: bsky_images,
            })
        } else {
            None
        };

        let facets = Self::detect_facets(message);

        let record = BlueskyPost {
            record_type: "app.bsky.feed.post".to_string(),
            text: message.to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            facets: if facets.is_empty() {
                None
            } else {
                Some(facets)
            },
            embed,
        };

        let body = CreateRecordRequest {
            repo: session.did.clone(),
            collection: "app.bsky.feed.post".to_string(),
            record,
        };

        let response = self
            .client
            .post(format!(
                "https://{}/xrpc/com.atproto.repo.createRecord",
                self.host
            ))
            .bearer_auth(&session.access_jwt)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Bluesky API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Bluesky post failed: {}",
                error_text
            )));
        }

        let record_response: CreateRecordResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Bluesky response: {}", e)))?;

        let rkey = record_response
            .uri
            .rsplit('/')
            .next()
            .unwrap_or(&record_response.cid);

        let url = format!("https://bsky.app/profile/{}/post/{}", session.handle, rkey);

        Ok(PostResponse {
            id: record_response.uri,
            url: Some(url),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        match self.create_session_fresh().await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_validates_credentials() {
        assert!(BlueskyStrategy::new(BlueskyCredentials {
            identifier: String::new(),
            password: "pass".to_string(),
            host: None,
        })
        .is_err());

        assert!(BlueskyStrategy::new(BlueskyCredentials {
            identifier: "user".to_string(),
            password: String::new(),
            host: None,
        })
        .is_err());

        let s = BlueskyStrategy::new(BlueskyCredentials {
            identifier: "user.bsky.social".to_string(),
            password: "app-password".to_string(),
            host: None,
        })
        .unwrap();
        assert_eq!(s.host, "bsky.social");

        let s = BlueskyStrategy::new(BlueskyCredentials {
            identifier: "user".to_string(),
            password: "pass".to_string(),
            host: Some("custom.pds.example".to_string()),
        })
        .unwrap();
        assert_eq!(s.host, "custom.pds.example");
    }

    #[test]
    fn test_calculate_message_length() {
        let s = BlueskyStrategy::new(BlueskyCredentials {
            identifier: "u".to_string(),
            password: "p".to_string(),
            host: None,
        })
        .unwrap();

        assert_eq!(s.calculate_message_length("hello"), 5);
        assert_eq!(
            s.calculate_message_length("check https://example.com/very/long/url/path/here out"),
            37
        );
    }

    #[test]
    fn test_detect_facets() {
        let facets = BlueskyStrategy::detect_facets("hello https://example.com world");
        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].index.byte_start, 6);
        assert_eq!(facets[0].index.byte_end, 25);
    }

    #[test]
    fn test_strategy_metadata() {
        let s = BlueskyStrategy::new(BlueskyCredentials {
            identifier: "u".to_string(),
            password: "p".to_string(),
            host: None,
        })
        .unwrap();
        assert_eq!(s.id(), "bluesky");
        assert_eq!(s.name(), "Bluesky");
        assert_eq!(s.max_message_length(), 300);
    }
}
