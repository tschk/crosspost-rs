use crate::env::required_env;
use crate::error::{Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{InstagramCredentials, PostOptions};
use serde::Deserialize;

/// Strategy for posting to Instagram via Graph API.
///
/// Requires a business or creator account. Images must have a public `image_url`
/// set on `ImageEmbed` since Instagram does not accept direct binary uploads.
pub struct InstagramStrategy {
    client: reqwest::Client,
    credentials: InstagramCredentials,
    api_base: String,
}

impl InstagramStrategy {
    pub fn new(credentials: InstagramCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "Instagram access_token is required".to_string(),
            ));
        }
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Platform(format!("Failed to build HTTP client: {}", e)))?,
            credentials,
            api_base: "https://graph.facebook.com/v18.0".to_string(),
        })
    }

    /// Override the API base URL (for testing with mock servers).
    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(InstagramCredentials {
            access_token: required_env("INSTAGRAM_ACCESS_TOKEN")?,
        })
    }

    /// Create a media container and publish it (Instagram's two-step flow).
    async fn create_and_publish(
        &self,
        container_params: serde_json::Value,
    ) -> Result<PostResponse> {
        // Step 1: Create media container
        let response = self
            .client
            .post(format!("{}/me/media", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .json(&container_params)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Instagram API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Instagram container creation failed: {}",
                error_text
            )));
        }

        let container: InstagramPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Instagram response: {}", e)))?;

        // Step 2: Publish the container
        let publish_body = serde_json::json!({
            "creation_id": container.id,
        });

        let response = self
            .client
            .post(format!("{}/me/media_publish", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .json(&publish_body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Instagram API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "Instagram publish failed: {}",
                error_text
            )));
        }

        let published: InstagramPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Instagram response: {}", e)))?;

        Ok(PostResponse {
            id: published.id.clone(),
            url: Some(format!("https://instagram.com/p/{}", published.id)),
        })
    }
}

#[derive(Deserialize)]
struct InstagramPostResponse {
    id: String,
}

#[async_trait::async_trait]
impl Strategy for InstagramStrategy {
    fn name(&self) -> &str {
        "Instagram"
    }

    fn id(&self) -> &str {
        "instagram"
    }

    fn max_message_length(&self) -> usize {
        2200
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let images = get_images(options);

        if images.len() == 1 {
            // Single image post
            let img = &images[0];
            let image_url = img.image_url.as_ref().ok_or_else(|| {
                Error::Platform(
                    "Instagram requires images to have a public image_url. Direct binary upload is not supported.".to_string(),
                )
            })?;

            let container_params = serde_json::json!({
                "image_url": image_url,
                "caption": message,
            });

            return self.create_and_publish(container_params).await;
        }

        if images.len() > 1 {
            // Multi-image carousel post
            let mut children_ids = Vec::new();
            for img in images.iter().take(10) {
                let image_url = img.image_url.as_ref().ok_or_else(|| {
                    Error::Platform(
                        "Instagram requires images to have a public image_url. Direct binary upload is not supported.".to_string(),
                    )
                })?;

                let child_params = serde_json::json!({
                    "image_url": image_url,
                    "is_carousel_item": true,
                });

                let response = self
                    .client
                    .post(format!("{}/me/media", self.api_base))
                    .bearer_auth(&self.credentials.access_token)
                    .json(&child_params)
                    .send()
                    .await
                    .map_err(|e| Error::Platform(format!("Instagram API error: {}", e)))?;

                if !response.status().is_success() {
                    let error_text = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Unknown error".to_string());
                    return Err(Error::Platform(format!(
                        "Instagram carousel item creation failed: {}",
                        error_text
                    )));
                }

                let child: InstagramPostResponse = response.json().await.map_err(|e| {
                    Error::Platform(format!("Failed to parse Instagram response: {}", e))
                })?;
                children_ids.push(child.id);
            }

            let container_params = serde_json::json!({
                "media_type": "CAROUSEL",
                "children": children_ids,
                "caption": message,
            });

            return self.create_and_publish(container_params).await;
        }

        // Text-only post (no images) - use simple container flow
        let container_params = serde_json::json!({
            "caption": message,
        });

        self.create_and_publish(container_params).await
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/me", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Instagram API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ImageEmbed;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> InstagramStrategy {
        InstagramStrategy::new(InstagramCredentials {
            access_token: "test-token".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(InstagramStrategy::new(InstagramCredentials {
            access_token: String::new(),
        })
        .is_err());

        assert!(InstagramStrategy::new(InstagramCredentials {
            access_token: "token".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = test_strategy();
        assert_eq!(s.id(), "instagram");
        assert_eq!(s.name(), "Instagram");
        assert_eq!(s.max_message_length(), 2200);
    }

    #[tokio::test]
    async fn test_post_with_image_url() {
        let mock_server = MockServer::start().await;

        // Mock container creation
        Mock::given(method("POST"))
            .and(path("/me/media"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"id": "container-123"})),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        // Mock publish
        Mock::given(method("POST"))
            .and(path("/me/media_publish"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "post-456"})),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());

        let options = PostOptions {
            images: vec![ImageEmbed {
                data: vec![],
                alt: None,
                mime_type: None,
                image_url: Some("https://example.com/photo.jpg".to_string()),
            }],
        };

        let result = strategy.post("Test caption", Some(&options)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.id, "post-456");
    }

    #[tokio::test]
    async fn test_post_rejects_binary_only_images() {
        let strategy = test_strategy();

        let options = PostOptions {
            images: vec![ImageEmbed {
                data: vec![0xFF, 0xD8], // JPEG header bytes
                alt: None,
                mime_type: Some("image/jpeg".to_string()),
                image_url: None, // No URL
            }],
        };

        let result = strategy.post("Test", Some(&options)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("image_url"));
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "12345"})),
            )
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
            .and(path("/me"))
            .respond_with(ResponseTemplate::new(401))
            .expect(1)
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(!strategy.validate_credentials().await.unwrap());
    }
}
