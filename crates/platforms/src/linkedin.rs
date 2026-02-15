use crate::platform_trait::{Platform, PostRequest, PostResponse};
use crosspost_core::{Error, Result};
use serde::{Deserialize, Serialize};

pub struct LinkedInClient {
    client: reqwest::Client,
}

impl LinkedInClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for LinkedInClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct LinkedInPostRequest {
    author: String,
    #[serde(rename = "lifecycleState")]
    lifecycle_state: String,
    #[serde(rename = "specificContent")]
    specific_content: LinkedInSpecificContent,
    visibility: LinkedInVisibility,
}

#[derive(Serialize)]
struct LinkedInSpecificContent {
    #[serde(rename = "com.linkedin.ugc.ShareContent")]
    share_content: LinkedInShareContent,
}

#[derive(Serialize)]
struct LinkedInShareContent {
    #[serde(rename = "shareCommentary")]
    share_commentary: LinkedInShareCommentary,
    #[serde(rename = "shareMediaCategory")]
    share_media_category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    media: Option<Vec<LinkedInMedia>>,
}

#[derive(Serialize)]
struct LinkedInMedia {
    status: String,
    #[serde(rename = "originalUrl")]
    #[serde(skip_serializing_if = "Option::is_none")]
    original_url: Option<String>,
    media: Option<String>,
    title: Option<LinkedInMediaTitle>,
}

#[derive(Serialize)]
struct LinkedInMediaTitle {
    text: String,
}

#[derive(Deserialize)]
struct LinkedInRegisterUploadResponse {
    value: LinkedInRegisterUploadValue,
}

#[derive(Deserialize)]
struct LinkedInRegisterUploadValue {
    asset: String,
    #[serde(rename = "uploadMechanism")]
    upload_mechanism: serde_json::Value,
}

#[derive(Serialize)]
struct LinkedInShareCommentary {
    text: String,
}

#[derive(Serialize)]
struct LinkedInVisibility {
    #[serde(rename = "com.linkedin.ugc.MemberNetworkVisibility")]
    member_network_visibility: String,
}

#[derive(Deserialize)]
struct LinkedInPostResponse {
    id: String,
}

#[derive(Deserialize)]
struct LinkedInProfileResponse {
    sub: String,
}

#[async_trait::async_trait]
impl Platform for LinkedInClient {
    async fn post(&self, access_token: &str, request: PostRequest) -> Result<PostResponse> {
        // First get the user's profile URN
        let profile = self
            .client
            .get("https://api.linkedin.com/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("LinkedIn API error: {}", e)))?;

        if !profile.status().is_success() {
            let error_text = profile
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "LinkedIn profile fetch error: {}",
                error_text
            )));
        }

        let profile_data: LinkedInProfileResponse = profile
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse LinkedIn profile: {}", e)))?;

        let author_urn = format!("urn:li:person:{}", profile_data.sub);

        // Upload images if present
        let (media_category, media_list) = if let Some(ref images) = request.images {
            if !images.is_empty() {
                let mut media_entries = Vec::new();
                for img in images.iter().take(4) {
                    // Step 1: Register upload
                    let register_body = serde_json::json!({
                        "registerUploadRequest": {
                            "recipes": ["urn:li:digitalmediaRecipe:feedshare-image"],
                            "owner": author_urn,
                            "serviceRelationships": [{
                                "relationshipType": "OWNER",
                                "identifier": "urn:li:userGeneratedContent"
                            }]
                        }
                    });

                    let register_resp = self
                        .client
                        .post("https://api.linkedin.com/v2/assets?action=registerUpload")
                        .bearer_auth(access_token)
                        .json(&register_body)
                        .send()
                        .await
                        .map_err(|e| {
                            Error::Platform(format!("LinkedIn upload register error: {}", e))
                        })?;

                    if !register_resp.status().is_success() {
                        let error_text = register_resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        return Err(Error::Platform(format!(
                            "LinkedIn upload register failed: {}",
                            error_text
                        )));
                    }

                    let register_data: LinkedInRegisterUploadResponse =
                        register_resp.json().await.map_err(|e| {
                            Error::Platform(format!("Failed to parse register response: {}", e))
                        })?;

                    // Step 2: Upload binary data
                    let upload_url = register_data
                        .value
                        .upload_mechanism
                        .get("com.linkedin.digitalmedia.uploading.MediaUploadHttpRequest")
                        .and_then(|m| m.get("uploadUrl"))
                        .and_then(|u| u.as_str())
                        .ok_or_else(|| {
                            Error::Platform("Missing upload URL from LinkedIn".to_string())
                        })?
                        .to_string();

                    let upload_resp = self
                        .client
                        .put(&upload_url)
                        .bearer_auth(access_token)
                        .header("Content-Type", "application/octet-stream")
                        .body(img.data.clone())
                        .send()
                        .await
                        .map_err(|e| Error::Platform(format!("LinkedIn upload error: {}", e)))?;

                    if !upload_resp.status().is_success() {
                        let error_text = upload_resp
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        return Err(Error::Platform(format!(
                            "LinkedIn image upload failed: {}",
                            error_text
                        )));
                    }

                    media_entries.push(LinkedInMedia {
                        status: "READY".to_string(),
                        original_url: None,
                        media: Some(register_data.value.asset),
                        title: img
                            .alt
                            .as_ref()
                            .map(|a| LinkedInMediaTitle { text: a.clone() }),
                    });
                }
                ("IMAGE".to_string(), Some(media_entries))
            } else {
                ("NONE".to_string(), None)
            }
        } else {
            ("NONE".to_string(), None)
        };

        let body = LinkedInPostRequest {
            author: author_urn,
            lifecycle_state: "PUBLISHED".to_string(),
            specific_content: LinkedInSpecificContent {
                share_content: LinkedInShareContent {
                    share_commentary: LinkedInShareCommentary {
                        text: request.content,
                    },
                    share_media_category: media_category,
                    media: media_list,
                },
            },
            visibility: LinkedInVisibility {
                member_network_visibility: "PUBLIC".to_string(),
            },
        };

        let response = self
            .client
            .post("https://api.linkedin.com/v2/ugcPosts")
            .bearer_auth(access_token)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("LinkedIn API error: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Platform(format!(
                "LinkedIn API error: {}",
                error_text
            )));
        }

        let li_response: LinkedInPostResponse = response
            .json()
            .await
            .map_err(|e| Error::Platform(format!("Failed to parse LinkedIn response: {}", e)))?;

        Ok(PostResponse {
            platform_post_id: li_response.id.clone(),
            url: Some(format!(
                "https://www.linkedin.com/feed/update/{}",
                li_response.id
            )),
        })
    }

    async fn validate_token(&self, access_token: &str) -> Result<bool> {
        let response = self
            .client
            .get("https://api.linkedin.com/v2/userinfo")
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| Error::Platform(format!("LinkedIn API error: {}", e)))?;

        Ok(response.status().is_success())
    }

    fn platform_name(&self) -> &'static str {
        "linkedin"
    }

    fn max_message_length(&self) -> usize {
        3000
    }
}
