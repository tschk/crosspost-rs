use crosspost_core::{Error, Result};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: Uuid,
    /// Tenant ID
    pub tenant_id: Uuid,
    /// User email
    pub email: String,
    /// Issued at (unix timestamp)
    pub iat: u64,
    /// Expiration (unix timestamp)
    pub exp: u64,
}

pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    expiry_secs: u64,
}

impl JwtManager {
    pub fn new(secret: &str, expiry_secs: u64) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            expiry_secs,
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user_id: Uuid, tenant_id: Uuid, email: &str) -> Result<String> {
        let now = chrono::Utc::now().timestamp() as u64;

        let claims = Claims {
            sub: user_id,
            tenant_id,
            email: email.to_string(),
            iat: now,
            exp: now + self.expiry_secs,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| Error::Auth(format!("Failed to generate token: {}", e)))
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let token_data = decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map_err(|e| Error::Unauthorized(format!("Invalid token: {}", e)))?;

        Ok(token_data.claims)
    }
}
