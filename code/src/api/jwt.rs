//! JWT Authentication

use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

use super::error::ApiError;

static JWT_SECRET: OnceLock<String> = OnceLock::new();

fn get_jwt_secret() -> &'static str {
    JWT_SECRET.get_or_init(|| {
        match std::env::var("AION_HIVE_JWT_SECRET") {
            Ok(secret) if !secret.is_empty() => secret,
            _ => {
                // 生产环境未设置密钥时使用随机密钥 + 告警日志
                // 注意：随机密钥意味着服务重启后所有旧 token 失效
                let fallback = format!("auto_generated_{}", Uuid::new_v4());
                tracing::error!(
                    "AION_HIVE_JWT_SECRET 未设置！已生成随机密钥。生产环境请务必通过环境变量配置固定密钥，否则每次重启后所有 JWT token 将失效。"
                );
                fallback
            }
        }
    })
}

fn get_jwt_expiry_hours() -> i64 {
    std::env::var("AION_HIVE_JWT_EXPIRY_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(24)
}

/// 认证来源，区分 JWT 的签发途径
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthSource {
    /// 用户/管理员直接登录（subject = identity_id UUID）
    UserLogin,
    /// Admin 登录
    AdminLogin,
    /// 通过 API Key 注册的 Agent（subject = agent_id UUID）
    RegisteredAgent,
    /// 旧的 agent（subject = agent_id string，向后兼容）
    LegacyAgent,
}

impl Default for AuthSource {
    fn default() -> Self {
        AuthSource::LegacyAgent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub subject: String,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
    /// 归属 identity 的 UUID（新字段，旧 token 默认为空）
    #[serde(default)]
    pub identity_id: String,
    /// 认证来源
    #[serde(default)]
    pub auth_source: AuthSource,
    /// Agent 调用时携带的自定义名称
    #[serde(default)]
    pub agent_name: Option<String>,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Clone)]
pub struct AgentContext {
    pub subject: String,
    pub roles: Vec<String>,
    pub scope: Vec<String>,
    /// 归属的 identity UUID（从 JWT claims 解析）
    pub identity_id: Option<Uuid>,
    /// 调用方 agent 的 UUID（仅 RegisteredAgent 来源时有值）
    pub agent_id: Option<Uuid>,
    /// 当前 MCP session UUID（MCP 连接时自动创建）
    pub session_id: Option<Uuid>,
    /// API key 关联的组织 UUID（API key 认证时自动填充）
    pub org_id: Option<Uuid>,
    /// 本次请求使用的 API key UUID（用于审计追踪）
    pub api_key_id: Option<Uuid>,
    /// 认证来源
    pub auth_source: AuthSource,
    /// Agent 名称
    pub agent_name: Option<String>,
    /// 原始 API key 明文（仅 HTTP/SSE 模式下 API key 认证时填充，
    /// 用于在 cli.setup 等场景生成 config.toml。stdio 模式下此字段为空。）
    pub raw_api_key: Option<String>,
}

pub struct JwtAuth;

impl AgentContext {
    pub fn new(subject: String) -> Self {
        Self {
            subject,
            roles: vec![],
            scope: vec![],
            identity_id: None,
            agent_id: None,
            session_id: None,
            org_id: None,
            api_key_id: None,
            auth_source: AuthSource::LegacyAgent,
            agent_name: None,
            raw_api_key: None,
        }
    }

    pub fn from_claims(claims: Claims) -> Self {
        let identity_id = if claims.identity_id.is_empty() {
            None
        } else {
            Uuid::parse_str(&claims.identity_id).ok()
        };

        let agent_id = if claims.auth_source == AuthSource::RegisteredAgent {
            Uuid::parse_str(&claims.subject).ok()
        } else {
            None
        };

        Self {
            subject: claims.subject,
            roles: claims.roles,
            scope: claims.scope,
            identity_id,
            agent_id,
            session_id: None,
            org_id: None,
            api_key_id: None,
            auth_source: claims.auth_source,
            agent_name: claims.agent_name,
            raw_api_key: None,
        }
    }

    pub fn require_admin(&self) -> Result<(), ApiError> {
        if !self.roles.iter().any(|r| r == "admin") {
            return Err(ApiError::Unauthorized("Admin access required".to_string()));
        }
        Ok(())
    }

    pub fn require_identity(&self) -> Result<Uuid, ApiError> {
        self.identity_id
            .ok_or_else(|| ApiError::Unauthorized("Identity not found in token".to_string()))
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
        identity_id: String::new(),
        auth_source: AuthSource::default(),
        agent_name: None,
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

/// 生成管理员/用户登录 Token
/// Phase 2: JWT 瘦身 - 不再通过 roles 携带权限信息，统一走 PermissionService
pub fn generate_identity_token(
    identity_id: Uuid,
    roles: &[&str],
    scope: &[&str],
) -> Result<String, ApiError> {
    let now = Utc::now();
    let exp = now + Duration::hours(get_jwt_expiry_hours());

    // Phase 2: 始终使用 UserLogin，不再区分 AdminLogin/UserLogin
    // 权限判断统一走 PermissionService::has_permission()
    let auth_source = AuthSource::UserLogin;

    let claims = Claims {
        subject: identity_id.to_string(),
        roles: roles.iter().map(|s| s.to_string()).collect(),
        scope: scope.iter().map(|s| s.to_string()).collect(),
        identity_id: identity_id.to_string(),
        auth_source,
        agent_name: None,
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
        identity_id: String::new(),
        auth_source: AuthSource::default(),
        agent_name: None,
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
        return Err(ApiError::Unauthorized("Invalid token purpose".to_string()));
    }
    Ok(claims.subject)
}

/// Hash a token string for storage (SHA-256)
pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// 判断一个 Bearer token 是否看起来像 API key（sk_ 前缀）
pub fn is_api_key_format(token: &str) -> bool {
    token.starts_with("sk_")
}

/// 从 Identity 信息直接构建 AgentContext（仅 API Key 验证，无需 Agent 注册/JWT）
pub fn agent_context_from_identity(
    identity_id: Uuid,
    identity_name: &str,
    session_id: Option<Uuid>,
    org_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
    raw_api_key: Option<String>,
) -> AgentContext {
    AgentContext {
        subject: identity_id.to_string(),
        roles: vec![],
        scope: vec![],
        identity_id: Some(identity_id),
        agent_id: None,
        session_id,
        org_id,
        api_key_id,
        auth_source: AuthSource::RegisteredAgent,
        agent_name: Some(identity_name.to_string()),
        raw_api_key,
    }
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

        Ok(AgentContext::from_claims(claims))
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
    fn test_identity_token() {
        let identity_id = Uuid::new_v4();
        let token = generate_identity_token(identity_id, &["admin"], &[]).unwrap();
        let claims = verify_token(&token).unwrap();
        let ctx = AgentContext::from_claims(claims);

        assert_eq!(ctx.identity_id, Some(identity_id));
        assert_eq!(ctx.auth_source, AuthSource::AdminLogin);
        assert_eq!(ctx.agent_id, None);
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
        assert_eq!(ctx.identity_id, None);
    }

    #[test]
    fn test_require_identity() {
        let ctx_no_identity = AgentContext::new("user-1".to_string());
        assert!(ctx_no_identity.require_identity().is_err());

        let ctx_with_identity = AgentContext {
            identity_id: Some(Uuid::new_v4()),
            ..AgentContext::new("user-1".to_string())
        };
        assert!(ctx_with_identity.require_identity().is_ok());
    }

    #[test]
    fn test_token_hash() {
        let hash1 = hash_token("test-token-123");
        let hash2 = hash_token("test-token-123");
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_legacy_token_backward_compat() {
        // Old-style token without identity_id/auth_source should still work
        let token = generate_token("old-agent", &[], &[]).unwrap();
        let claims = verify_token(&token).unwrap();
        let ctx = AgentContext::from_claims(claims);

        assert_eq!(ctx.subject, "old-agent");
        assert_eq!(ctx.identity_id, None);
        assert_eq!(ctx.agent_id, None);
    }
}
