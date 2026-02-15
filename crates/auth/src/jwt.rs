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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_jwt_manager() -> JwtManager {
        JwtManager::new("test-secret-key-at-least-32-bytes-long!", 3600)
    }

    #[test]
    fn test_generate_and_validate_token() {
        let jwt = test_jwt_manager();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = jwt.generate_token(user_id, tenant_id, email).unwrap();
        let claims = jwt.validate_token(&token).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.tenant_id, tenant_id);
        assert_eq!(claims.email, email);
    }

    #[test]
    fn test_invalid_token() {
        let jwt = test_jwt_manager();
        let result = jwt.validate_token("invalid.token.here");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_secret() {
        let jwt1 = JwtManager::new("secret-one-is-long-enough-for-testing!", 3600);
        let jwt2 = JwtManager::new("secret-two-is-long-enough-for-testing!", 3600);

        let token = jwt1
            .generate_token(Uuid::new_v4(), Uuid::new_v4(), "test@test.com")
            .unwrap();
        let result = jwt2.validate_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_expired_token() {
        // Create a token that is already expired (jsonwebtoken has 60s default leeway)
        let jwt = test_jwt_manager();
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        // Manually create an expired token by setting exp in the past
        let claims = Claims {
            sub: user_id,
            tenant_id,
            email: "test@test.com".to_string(),
            iat: 1000,
            exp: 1000, // far in the past
        };

        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret(
                "test-secret-key-at-least-32-bytes-long!".as_bytes(),
            ),
        )
        .unwrap();

        let result = jwt.validate_token(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_claims_serialization() {
        let claims = Claims {
            sub: Uuid::new_v4(),
            tenant_id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            iat: 1000,
            exp: 2000,
        };

        let json = serde_json::to_string(&claims).unwrap();
        let parsed: Claims = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sub, claims.sub);
        assert_eq!(parsed.tenant_id, claims.tenant_id);
        assert_eq!(parsed.email, claims.email);
    }
}
