use crate::env::required_env;
use crate::error::{platform_response_error, Error, Result};
use crate::strategy::{get_images, PostResponse, Strategy};
use crate::types::{FacebookCredentials, PostOptions};
use serde::Deserialize;

/// Strategy for posting to Facebook Pages via Graph API.
///
/// Supports single and multi-image posts.
pub struct FacebookStrategy {
    client: reqwest::Client,
    credentials: FacebookCredentials,
    api_base: String,
}

impl FacebookStrategy {
    pub fn new(credentials: FacebookCredentials) -> Result<Self> {
        if credentials.access_token.is_empty() {
            return Err(Error::Validation(
                "Facebook access_token is required".to_string(),
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

    #[cfg(test)]
    pub(crate) fn with_api_base(mut self, base: String) -> Self {
        self.api_base = base;
        self
    }

    pub fn from_env() -> Result<Self> {
        Self::new(FacebookCredentials {
            access_token: required_env("FACEBOOK_ACCESS_TOKEN")?,
        })
    }

    async fn post_with_images(
        &self,
        caption: &str,
        images: &[crate::types::ImageEmbed],
    ) -> Result<PostResponse> {
        if images.len() == 1 {
            let img = &images[0];
            let mime = img.mime_type.as_deref().unwrap_or("image/jpeg").to_string();
            let part = reqwest::multipart::Part::bytes(img.data.clone())
                .file_name("image")
                .mime_str(&mime)
                .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?;
            let form = reqwest::multipart::Form::new()
                .text("caption", caption.to_string())
                .part("source", part);

            let response = self
                .client
                .post(format!("{}/me/photos", self.api_base))
                .bearer_auth(&self.credentials.access_token)
                .multipart(form)
                .send()
                .await
                .map_err(|e| Error::Platform(format!("Facebook photo upload error: {}", e)))?;

            if !response.status().is_success() {
                return Err(platform_response_error(self.name(), response).await);
            }

            let fb_response: FacebookPostResponse = response.json().await.map_err(|e| {
                Error::Platform(format!("Failed to parse Facebook response: {}", e))
            })?;

            return Ok(PostResponse {
                id: fb_response.id.clone(),
                url: Some(format!("https://facebook.com/{}", fb_response.id)),
            });
        }

        // Multiple images: upload each as unpublished, then create feed post
        let mut media_ids = Vec::new();
        for img in images.iter().take(4) {
            let mime = img.mime_type.as_deref().unwrap_or("image/jpeg").to_string();
            let part = reqwest::multipart::Part::bytes(img.data.clone())
                .file_name("image")
                .mime_str(&mime)
                .map_err(|e| Error::Platform(format!("Invalid MIME type: {}", e)))?;
            let form = reqwest::multipart::Form::new()
                .text("published", "false".to_string())
                .part("source", part);

            let response = self
                .client
                .post(format!("{}/me/photos", self.api_base))
                .bearer_auth(&self.credentials.access_token)
                .multipart(form)
                .send()
                .await
                .map_err(|e| Error::Platform(format!("Facebook photo upload error: {}", e)))?;

            if !response.status().is_success() {
                return Err(platform_response_error(self.name(), response).await);
            }

            let fb_response: FacebookPostResponse = response.json().await.map_err(|e| {
                Error::Platform(format!("Failed to parse Facebook response: {}", e))
            })?;

            media_ids.push(fb_response.id);
        }

        let mut query_params: Vec<(String, String)> =
            vec![("message".to_string(), caption.to_string())];
        for (i, media_id) in media_ids.iter().enumerate() {
            query_params.push((
                format!("attached_media[{}]", i),
                format!("{{\"media_fbid\":\"{}\"}}", media_id),
            ));
        }

        let response = self
            .client
            .post(format!("{}/me/feed", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .query(&query_params)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        if !response.status().is_success() {
            return Err(platform_response_error(self.name(), response).await);
        }

        let fb_response: FacebookPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Facebook response: {}", e)))?;

        Ok(PostResponse {
            id: fb_response.id.clone(),
            url: Some(format!("https://facebook.com/{}", fb_response.id)),
        })
    }
}

#[derive(Deserialize)]
struct FacebookPostResponse {
    id: String,
}

#[async_trait::async_trait]
impl Strategy for FacebookStrategy {
    fn name(&self) -> &str {
        "Facebook"
    }

    fn id(&self) -> &str {
        "facebook"
    }

    fn max_message_length(&self) -> usize {
        63206
    }

    async fn post(&self, message: &str, options: Option<&PostOptions>) -> Result<PostResponse> {
        let images = get_images(options);
        if !images.is_empty() {
            return self.post_with_images(message, images).await;
        }

        let body = serde_json::json!({
            "message": message,
        });

        let response = self
            .client
            .post(format!("{}/me/feed", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        if !response.status().is_success() {
            return Err(platform_response_error(self.name(), response).await);
        }

        let fb_response: FacebookPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse Facebook response: {}", e)))?;

        Ok(PostResponse {
            id: fb_response.id.clone(),
            url: Some(format!("https://facebook.com/{}", fb_response.id)),
        })
    }

    async fn validate_credentials(&self) -> Result<bool> {
        let response = self
            .client
            .get(format!("{}/me", self.api_base))
            .bearer_auth(&self.credentials.access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("Facebook API error: {}", e)))?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Strategy;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_strategy() -> FacebookStrategy {
        FacebookStrategy::new(FacebookCredentials {
            access_token: "test-token".to_string(),
        })
        .unwrap()
    }

    #[test]
    fn test_new_validates_credentials() {
        assert!(FacebookStrategy::new(FacebookCredentials {
            access_token: String::new(),
        })
        .is_err());

        assert!(FacebookStrategy::new(FacebookCredentials {
            access_token: "token".to_string(),
        })
        .is_ok());
    }

    #[test]
    fn test_strategy_metadata() {
        let s = FacebookStrategy::new(FacebookCredentials {
            access_token: "t".to_string(),
        })
        .unwrap();
        assert_eq!(s.id(), "facebook");
        assert_eq!(s.name(), "Facebook");
        assert_eq!(s.max_message_length(), 63206);
    }

    #[tokio::test]
    async fn test_post_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/me/feed"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "12345_67890"})),
            )
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let response = strategy.post("Hello Facebook!", None).await.unwrap();
        assert_eq!(response.id, "12345_67890");
        assert_eq!(
            response.url,
            Some("https://facebook.com/12345_67890".to_string())
        );
    }

    #[tokio::test]
    async fn test_post_api_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/me/feed"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        let result = strategy.post("Hello!", None).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("HTTP 403"), "Got: {}", err);
    }

    #[tokio::test]
    async fn test_validate_credentials_success() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/me"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": "123"})),
            )
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
            .mount(&mock_server)
            .await;

        let strategy = test_strategy().with_api_base(mock_server.uri());
        assert!(!strategy.validate_credentials().await.unwrap());
    }
}
