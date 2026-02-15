use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub struct BlueskyClient {
    client: reqwest::Client,
}

impl BlueskyClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for BlueskyClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct CreateSessionRequest {
    identifier: String,
    password: String,
}

#[derive(Deserialize)]
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

impl BlueskyClient {
    /// Parse access_token as "identifier|password"
    fn parse_credentials(access_token: &str) -> Result<(&str, &str)> {
        let parts: Vec<&str> = access_token.splitn(2, '|').collect();
        if parts.len() != 2 {
            return Err(Error::Platform(
                "Bluesky requires credentials as 'identifier|app_password'".to_string(),
            ));
        }
        Ok((parts[0], parts[1]))
    }

    /// Create a session (login) and return JWT + DID + handle
    async fn create_session(
        &self,
        identifier: &str,
        password: &str,
    ) -> Result<CreateSessionResponse> {
        let body = CreateSessionRequest {
            identifier: identifier.to_string(),
            password: password.to_string(),
        };

        let response = self
            .client
            .post("https://bsky.social/xrpc/com.atproto.server.createSession")
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

    /// Upload a blob (image) to Bluesky
    async fn upload_blob(&self, jwt: &str, data: &[u8], mime_type: &str) -> Result<BlobRef> {
        let response = self
            .client
            .post("https://bsky.social/xrpc/com.atproto.repo.uploadBlob")
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

    /// Detect URL, @mention, and #hashtag facets in text
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

        // @mention detection (e.g., @user.bsky.social)
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

#[async_trait::async_trait]
impl Platform for BlueskyClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        let (identifier, password) = Self::parse_credentials(access_token)?;
        let session = self.create_session(identifier, password).await?;

        // Build embed if images are present
        let embed = if let Some(ref images) = request.images {
            if !images.is_empty() {
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
            }
        } else {
            None
        };

        let facets = Self::detect_facets(&request.content);

        let record = BlueskyPost {
            record_type: "app.bsky.feed.post".to_string(),
            text: request.content,
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
            .post("https://bsky.social/xrpc/com.atproto.repo.createRecord")
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

        // Extract rkey from URI: at://did:plc:xxx/app.bsky.feed.post/rkey
        let rkey = record_response
            .uri
            .rsplit('/')
            .next()
            .unwrap_or(&record_response.cid);

        let url = format!("https://bsky.app/profile/{}/post/{}", session.handle, rkey);

        Ok(PostResponse {
            platform_post_id: record_response.uri,
            url: Some(url),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let (identifier, password) = Self::parse_credentials(access_token)?;
        match self.create_session(identifier, password).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn platform_name(&self) -> &'static str {
        "bluesky"
    }

    fn max_message_length(&self) -> usize {
        300
    }

    fn calculate_message_length(&self, content: &str) -> usize {
        // Bluesky counts URLs > 27 chars as 27
        let mut length = 0;
        let mut remaining = content;

        while let Some(url_start) = remaining
            .find("http://")
            .or_else(|| remaining.find("https://"))
        {
            length += remaining[..url_start].len();
            let after_url = &remaining[url_start..];
            let url_end = after_url
                .find(|c: char| c.is_whitespace())
                .unwrap_or(after_url.len());
            let url_len = url_end;
            length += url_len.min(27);
            remaining = &after_url[url_end..];
        }
        length += remaining.len();
        length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_credentials() {
        let (id, pw) = BlueskyClient::parse_credentials("user.bsky.social|app-password").unwrap();
        assert_eq!(id, "user.bsky.social");
        assert_eq!(pw, "app-password");

        assert!(BlueskyClient::parse_credentials("invalid").is_err());
    }

    #[test]
    fn test_max_message_length() {
        let client = BlueskyClient::new();
        assert_eq!(client.max_message_length(), 300);
    }

    #[test]
    fn test_calculate_message_length() {
        let client = BlueskyClient::new();
        assert_eq!(client.calculate_message_length("hello"), 5);
        // URL > 27 chars counts as 27
        assert_eq!(
            client
                .calculate_message_length("check https://example.com/very/long/url/path/here out"),
            // "check " (6) + url counted as 27 + " out" (4) = 37
            37
        );
    }

    #[test]
    fn test_detect_facets() {
        let facets = BlueskyClient::detect_facets("hello https://example.com world");
        assert_eq!(facets.len(), 1);
        assert_eq!(facets[0].index.byte_start, 6);
        assert_eq!(facets[0].index.byte_end, 25);
    }
}
