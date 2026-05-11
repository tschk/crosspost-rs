use crate::env::required_env;
use crate::error::{platform_response_error, Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{LinkedInCredentials, PostOptions};
use serde::{Deserialize, Serialize};

const LINKEDIN_VERSION: &str = "202604";

/// Strategy for posting to LinkedIn via the Share API.
///
/// Supports image uploads via 3-step register/upload/post flow.
pub struct LinkedInStrategy {
    client: reqwest::Client,
    credentials: LinkedInCredentials,
    api_base: String,
}

impl LinkedInStrategy {
    pub fn new(credentials: LinkedInCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "LinkedIn access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: "https://api.linkedin.com".to_string(),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(LinkedInCredentials {
            access_token: required_env("LINKEDIN_ACCESS_TOKEN")?,
        })
    }

    fn rest_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "LinkedIn-Version",
            LINKEDIN_VERSION.parse().expect("static version header"),
        );
        headers.insert(
            "X-Restli-Protocol-Version",
            "2.0.0".parse().expect("static protocol header"),
        );
        headers
    }

    async fn initialize_image_upload(
        &self,
        author_urn: &str,
    ) -> Result<LinkedInInitializeUploadValue> {
        let body = serde_json::json!({
            "initializeUploadRequest": {
                "owner": author_urn,
            }
        });

        let response = self
            .client
            .post(format!(
                "{}/rest/images?action=initializeUpload",
                self.api_base
            ))
            .bearer_auth(&self.credentials.access_token)
            .headers(self.rest_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("LinkedIn image init error: {}", e)))?;

        if !response.status().is_success() {
            return Err(platform_response_error(self.name(), response).await);
        }

        let parsed: LinkedInInitializeUploadResponse = response.json().await.map_err(|e| {
            Error::Platform(format!(
                "Failed to parse LinkedIn image init response: {}",
                e
            ))
        })?;

        Ok(parsed.value)
    }

    async fn put_image_bytes(&self, upload_url: &str, data: &[u8]) -> Result<()> {
        let upload_resp = self
            .client
            .put(upload_url)
            .bearer_auth(&self.credentials.access_token)
            .header("Content-Type", "image/*")
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| Error::Platform(format!("LinkedIn upload error: {}", e)))?;

        if !upload_resp.status().is_success() {
            return Err(platform_response_error(self.name(), upload_resp).await);
        }

        Ok(())
    }
}

#[derive(Deserialize)]
struct LinkedInInitializeUploadResponse {
    value: LinkedInInitializeUploadValue,
}

#[derive(Deserialize)]
struct LinkedInInitializeUploadValue {
    image: String,
    #[serde(rename = "uploadUrl")]
    upload_url: String,
}

#[derive(Serialize)]
struct LinkedInPostsRequest {
    author: String,
    commentary: String,
    visibility: String,
    distribution: LinkedInDistribution,
    #[serde(rename = "lifecycleState")]
    lifecycle_state: String,
    #[serde(rename = "isReshareDisabledByAuthor")]
    is_reshare_disabled_by_author: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<LinkedInPostContent>,
}

#[derive(Serialize)]
struct LinkedInDistribution {
    #[serde(rename = "feedDistribution")]
    feed_distribution: String,
    #[serde(rename = "targetEntities")]
    target_entities: Vec<serde_json::Value>,
    #[serde(rename = "thirdPartyDistributionChannels")]
    third_party_distribution_channels: Vec<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum LinkedInPostContent {
    SingleImage {
        media: LinkedInSingleImage,
    },
    MultiImage {
        #[serde(rename = "multiImage")]
        multi_image: LinkedInMultiImageBlock,
    },
}

#[derive(Serialize)]
struct LinkedInSingleImage {
    id: String,
    #[serde(rename = "altText")]
    alt_text: String,
}

#[derive(Serialize)]
struct LinkedInMultiImageBlock {
    images: Vec<LinkedInMultiImageItem>,
}

#[derive(Serialize)]
struct LinkedInMultiImageItem {
    id: String,
    #[serde(rename = "altText")]
    alt_text: String,
}

#[derive(Deserialize)]
struct LinkedInProfileResponse {
    sub: String,
}

#[async_trait::async_trait]
impl Strategy for LinkedInStrategy {
    fn name(&self) -> &str {
        "LinkedIn"
    }

    fn id(&self) -> &str {
        "linkedin"
    }

    fn max_message_length(&self) -> usize {
        3000
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let profile = self
            .client
            .get(format!("{}/v2/userinfo", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("LinkedIn API error: {}", e)))?;

        if !profile.status().is_success() {
            return Err(platform_response_error(self.name(), profile).await);
        }

        let profile_data: LinkedInProfileResponse = profile
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse LinkedIn profile: {}", e)))?;

        let author_urn = format!("urn:li:person:{}", profile_data.sub);

        let images = get_images(options);
        let content = if images.is_empty() {
            None
        } else {
            let mut image_urns = Vec::new();
            let mut alts = Vec::new();
            for img in images.iter().take(4) {
                let init = self.initialize_image_upload(&author_urn).await?;
                self.put_image_bytes(&init.upload_url, &img.data).await?;
                image_urns.push(init.image);
                alts.push(img.alt.clone().unwrap_or_default());
            }

            if image_urns.len() == 1 {
                Some(LinkedInPostContent::SingleImage {
                    media: LinkedInSingleImage {
                        id: image_urns[0].clone(),
                        alt_text: alts[0].clone(),
                    },
                })
            } else {
                Some(LinkedInPostContent::MultiImage {
                    multi_image: LinkedInMultiImageBlock {
                        images: image_urns
                            .into_iter()
                            .zip(alts)
                            .map(|(id, alt_text)| LinkedInMultiImageItem { id, alt_text })
                            .collect(),
                    },
                })
            }
        };

        let body = LinkedInPostsRequest {
            author: author_urn,
            commentary: message.to_string(),
            visibility: "PUBLIC".to_string(),
            distribution: LinkedInDistribution {
                feed_distribution: "MAIN_FEED".to_string(),
                target_entities: Vec::new(),
                third_party_distribution_channels: Vec::new(),
            },
            lifecycle_state: "PUBLISHED".to_string(),
            is_reshare_disabled_by_author: false,
            content,
        };

        let response = self
            .client
            .post(format!("{}/rest/posts", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .headers(self.rest_headers())
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("LinkedIn API error: {}", e)))?;

        if !response.status().is_success() {
            return Err(platform_response_error(self.name(), response).await);
        }

        let post_id = response
            .headers()
            .get("x-restli-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                Error::Platform("LinkedIn post response missing x-restli-id header".to_string())
            })?;

        Ok(PostResponse {
            id: post_id.clone(),
            url: Some(format!("https://www.linkedin.com/feed/update/{}", post_id)),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/v2/userinfo", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("LinkedIn API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ImageEmbed;
    use crate::Strategy;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn test_strategy() -> LinkedInStrategy {
        LinkedInStrategy::new(LinkedInCredentials {
            access_token: "test-token".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(LinkedInStrategy::new(LinkedInCredentials {
            access_token: String::new(),
        })
        .is_err());

        assert!(LinkedInStrategy::new(LinkedInCredentials {
            access_token: "token".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = test_strategy();
        assert_eq!(s.id(), "linkedin");
        assert_eq!(s.name(), "LinkedIn");
        assert_eq!(s.max_message_length(), 3000);
    }

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sub": "abc123"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/rest/posts"))
            .respond_with(
                ResponseTemplate::new(201)
                    .append_header("x-restli-id", "urn:li:share:123")
                    .set_body_string(""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());

        let result = strategy.post("Hello LinkedIn!", None).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, "urn:li:share:123");
        assert_eq!(
            response.url.as_deref(),
            Some("https://www.linkedin.com/feed/update/urn:li:share:123")
        );
    }

    #[tokio::test]
    async fn test_post_with_one_image() {
        let mock_server = MockServer::start().await;
        let upload_path = "/mock-upload";

        Mock::given(method("GET"))
            .and(path("/v2/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sub": "abc123"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/rest/images"))
            .and(query_param("action", "initializeUpload"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "value": {
                    "image": "urn:li:image:999",
                    "uploadUrl": format!("{}{}", mock_server.uri(), upload_path)
                }
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("PUT"))
            .and(path(upload_path))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/rest/posts"))
            .respond_with(
                ResponseTemplate::new(201)
                    .append_header("x-restli-id", "urn:li:share:777")
                    .set_body_string(""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let opts = PostOptions {
            images: vec![ImageEmbed {
                data: vec![1, 2, 3],
                alt: Some("caption".into()),
                mime_type: None,
                image_url: None,
            }],
        };

        let result = strategy.post("Hello with pic", Some(&opts)).await.unwrap();
        assert_eq!(result.id, "urn:li:share:777");
    }

    #[tokio::test]
    async fn test_post_with_two_images() {
        let mock_server = MockServer::start().await;
        let upload_path = "/mock-upload";
        let base = mock_server.uri();
        let init_n = AtomicUsize::new(0);

        Mock::given(method("GET"))
            .and(path("/v2/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sub": "abc123"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/rest/images"))
            .and(query_param("action", "initializeUpload"))
            .respond_with(move |_req: &Request| {
                let i = init_n.fetch_add(1, Ordering::SeqCst);
                let image = if i == 0 {
                    "urn:li:image:111"
                } else {
                    "urn:li:image:222"
                };
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "value": {
                        "image": image,
                        "uploadUrl": format!("{}{}", base, upload_path)
                    }
                }))
            })
            .expect(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("PUT"))
            .and(path(upload_path))
            .respond_with(ResponseTemplate::new(200))
            .expect(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/rest/posts"))
            .respond_with(
                ResponseTemplate::new(201)
                    .append_header("x-restli-id", "urn:li:share:888")
                    .set_body_string(""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let opts = PostOptions {
            images: vec![
                ImageEmbed {
                    data: vec![1],
                    alt: Some("a".into()),
                    mime_type: None,
                    image_url: None,
                },
                ImageEmbed {
                    data: vec![2],
                    alt: Some("b".into()),
                    mime_type: None,
                    image_url: None,
                },
            ],
        };

        let result = strategy.post("Hi", Some(&opts)).await.unwrap();
        assert_eq!(result.id, "urn:li:share:888");
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sub": "abc123"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/rest/posts"))
            .respond_with(ResponseTemplate::new(422).set_body_string("Unprocessable Entity"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());

        let result = strategy.post("Hello!", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HTTP 422"));
    }

    #[tokio::test]
    async fn test_post_missing_restli_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sub": "abc123"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/rest/posts"))
            .respond_with(ResponseTemplate::new(201).set_body_string(""))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let err = strategy.post("Hello!", None).await.unwrap_err();
        assert!(err.to_string().contains("missing x-restli-id"), "{err}");
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/userinfo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sub": "abc123"
            })))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(strategy.validate_credentials().await.unwrap());
    }

    #[tokio::test]
    async fn test_validate_credentials_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v2/userinfo"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(!strategy.validate_credentials().await.unwrap());
    }
}
