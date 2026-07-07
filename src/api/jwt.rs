//! JWT Authentication

use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::error::ApiError;

fn get_jwt_secret() -> String {
    std::env::var("AION_HIVE_JWT_SECRET")
        .unwrap_or_else(|_| "aion_hive_secret_key_change_in_production".to_string())
}

fn get_jwt_expiry_hours() -> i64 {
    std::env::var("AION_HIVE_JWT_EXPIRY_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(24)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub subject: String,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub subject: String,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
}

pub struct JwtAuth;

impl AgentContext {
    pub fn new(subject: String) -> Self {
        Self {
            subject,
            roles: vec![],
            scope: vec![],
        }
    }

    pub fn require_admin(&self) -> Result<(), ApiError> {
        if !self.roles.iter().any(|r| r == "admin") {
            return Err(ApiError::Unauthorized("Admin access required".to_string()));
        }
        Ok(())
    }

    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    pub fn with_scope(mut self, scope: Vec<String>) -> Self {
        self.scope = scope;
        self
    }
}

pub fn generate_token(subject: &str, roles: &[&str], scope: &[&str]) -> Result<String, ApiError> {
    let now = Utc::now();
    let exp = now + Duration::hours(get_jwt_expiry_hours());

    let claims = Claims {
        subject: subject.to_string(),
        roles: roles.iter().map(|s| s.to_string()).collect(),
        scope: scope.iter().map(|s| s.to_string()).collect(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret().as_bytes()),
    )
    .map_err(|e| ApiError::InternalError(format!("Failed to generate token: {}", e)))
}

pub fn verify_token(token: &str) -> Result<Claims, ApiError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(get_jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .map_err(|e| ApiError::Unauthorized(format!("Invalid token: {}", e)))
}

/// Generate a short-lived token for password reset / email verification
pub fn generate_short_lived_token(
    subject: &str,
    purpose: &str,
    minutes: i64,
) -> Result<String, ApiError> {
    let now = Utc::now();
    let exp = now + Duration::minutes(minutes);

    let claims = Claims {
        subject: subject.to_string(),
        roles: vec![purpose.to_string()],
        scope: vec![],
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret().as_bytes()),
    )
    .map_err(|e| ApiError::InternalError(format!("Failed to generate token: {}", e)))
}

/// Verify a short-lived token and check the purpose role
pub fn verify_purpose_token(token: &str, purpose: &str) -> Result<String, ApiError> {
    let claims = verify_token(token)?;
    if !claims.roles.iter().any(|r| r == purpose) {
        return Err(ApiError::Unauthorized(format!("Invalid token purpose")));
    }
    Ok(claims.subject)
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AgentContext {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(ApiError::Unauthorized(
                "Invalid Authorization header format".to_string(),
            ));
        }

        let token = &auth_header[7..];
        let claims = verify_token(token)?;

        Ok(AgentContext {
            subject: claims.subject,
            roles: claims.roles,
            scope: claims.scope,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_token() {
        let token = generate_token("user-1", &["admin"], &["read"]).unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.subject, "user-1");
        assert_eq!(claims.roles, vec!["admin"]);
        assert_eq!(claims.scope, vec!["read"]);
    }

    #[test]
    fn test_invalid_token() {
        let result = verify_token("invalid_token");
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_context_builder() {
        let ctx = AgentContext::new("user-1".to_string())
            .with_roles(vec!["admin".to_string()])
            .with_scope(vec!["write".to_string()]);
        assert_eq!(ctx.subject, "user-1");
        assert_eq!(ctx.roles, vec!["admin"]);
        assert_eq!(ctx.scope, vec!["write"]);
    }
}
