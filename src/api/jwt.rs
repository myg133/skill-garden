//! JWT Authentication

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::ApiError;

fn get_jwt_secret() -> String {
    std::env::var("AION_HIVE_JWT_SECRET")
        .unwrap_or_else(|_| "aion_hive_secret_key_change_in_production".to_string())
}
const TOKEN_EXPIRY_HOURS: i64 = 24;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub agent_id: String,
    pub org_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub agent_id: String,
    pub org_id: Option<Uuid>,
    pub session_id: Option<Uuid>,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
}

pub struct JwtAuth;

impl AgentContext {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            org_id: None,
            session_id: None,
            roles: vec![],
            scope: vec![],
        }
    }

    pub fn with_org(mut self, org_id: Uuid) -> Self {
        self.org_id = Some(org_id);
        self
    }

    pub fn with_session(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);
        self
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

pub fn generate_token(agent_id: &str, roles: Vec<String>, scope: Vec<String>) -> Result<String, ApiError> {
    generate_token_with_context(agent_id, None, None, roles, scope)
}

pub fn generate_token_with_context(
    agent_id: &str,
    org_id: Option<Uuid>,
    session_id: Option<Uuid>,
    roles: Vec<String>,
    scope: Vec<String>,
) -> Result<String, ApiError> {
    let now = Utc::now();
    let exp = now + Duration::hours(TOKEN_EXPIRY_HOURS);

    let claims = Claims {
        agent_id: agent_id.to_string(),
        org_id,
        session_id,
        roles,
        scope,
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
            return Err(ApiError::Unauthorized("Invalid Authorization header format".to_string()));
        }

        let token = &auth_header[7..];
        let claims = verify_token(token)?;

        Ok(AgentContext {
            agent_id: claims.agent_id,
            org_id: claims.org_id,
            session_id: claims.session_id,
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
        let token = generate_token("agent-1", vec!["admin".to_string()], vec!["read".to_string()]).unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.agent_id, "agent-1");
        assert_eq!(claims.roles, vec!["admin"]);
        assert_eq!(claims.scope, vec!["read"]);
        assert!(claims.org_id.is_none());
        assert!(claims.session_id.is_none());
    }

    #[test]
    fn test_generate_token_with_context() {
        let org_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let token = generate_token_with_context(
            "agent-1",
            Some(org_id),
            Some(session_id),
            vec!["admin".to_string()],
            vec!["read".to_string()],
        ).unwrap();
        let claims = verify_token(&token).unwrap();
        assert_eq!(claims.agent_id, "agent-1");
        assert_eq!(claims.org_id, Some(org_id));
        assert_eq!(claims.session_id, Some(session_id));
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
        let org_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let ctx = AgentContext::new("agent-1".to_string())
            .with_org(org_id)
            .with_session(session_id)
            .with_roles(vec!["admin".to_string()])
            .with_scope(vec!["write".to_string()]);
        assert_eq!(ctx.agent_id, "agent-1");
        assert_eq!(ctx.org_id, Some(org_id));
        assert_eq!(ctx.session_id, Some(session_id));
        assert_eq!(ctx.roles, vec!["admin"]);
        assert_eq!(ctx.scope, vec!["write"]);
    }
}
