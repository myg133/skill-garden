//! Agent repository — support API Key-based agent registration with identity linking

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

/// Agent 记录（数据库模型）
#[derive(Debug, Clone)]
pub struct Agent {
    pub id: Uuid,
    pub agent_id: String,
    pub agent_secret_hash: String,
    pub agent_name: Option<String>,
    pub org_id: Option<Uuid>,
    pub capabilities: Vec<String>,
    pub roles: Vec<String>,
    /// 归属 identity
    pub identity_id: Option<Uuid>,
    /// 注册时使用的 API Key
    pub api_key_id: Option<Uuid>,
    /// Agent token 的 SHA-256 hash（新注册方式）
    pub agent_token_hash: Option<String>,
    /// Agent token 过期时间
    pub agent_token_expires_at: Option<DateTime<Utc>>,
    /// Agent 状态
    pub status: String,
    /// Agent 描述
    pub agent_description: Option<String>,
    /// 最后使用时间
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 旧版 Agent 注册（保留向后兼容）
pub struct NewAgent {
    pub agent_id: String,
    pub agent_secret: String,
    pub agent_name: Option<String>,
    pub org_id: Option<Uuid>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct AgentRepository {
    pool: PgPool,
}

impl AgentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    // ─── Agent CRUD ─────────────────────────────────────────

    /// 撤销 Agent（使其 token 失效）
    pub async fn revoke(&self, agent_id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE agents SET status = 'revoked', agent_token_hash = NULL, updated_at = NOW() WHERE agent_id = $1")
            .bind(agent_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(())
    }

    /// 按 identity_id 列出所有 Agent
    pub async fn list_by_identity(&self, identity_id: Uuid) -> DbResult<Vec<Agent>> {
        let agents = sqlx::query_as::<_, AgentRow>(
            r#"
            SELECT
                id, agent_id, agent_secret_hash, agent_name, org_id,
                capabilities, roles, identity_id, api_key_id,
                agent_token_hash, agent_token_expires_at, status,
                agent_description, last_used_at, created_at, updated_at
            FROM agents
            WHERE identity_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(identity_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(agents.into_iter().map(|a| a.into()).collect())
    }

    // ─── 旧版：向后兼容的 agent_secret 方法 ──────────────────────

    pub async fn create(&self, new_agent: NewAgent) -> DbResult<Agent> {
        let secret_hash = bcrypt::hash(&new_agent.agent_secret, bcrypt::DEFAULT_COST)
            .map_err(|e| DbError::ValidationError(format!("Failed to hash secret: {}", e)))?;

        let roles = Vec::<String>::new();
        let capabilities = new_agent.capabilities.unwrap_or_default();

        let agent = sqlx::query_as::<_, AgentRow>(
            r#"
            INSERT INTO agents (agent_id, agent_secret_hash, agent_name, roles, capabilities)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, agent_id, agent_secret_hash, agent_name, org_id,
                capabilities, roles, identity_id, api_key_id,
                agent_token_hash, agent_token_expires_at, status,
                agent_description, last_used_at, created_at, updated_at
            "#,
        )
        .bind(&new_agent.agent_id)
        .bind(&secret_hash)
        .bind(&new_agent.agent_name)
        .bind(&roles)
        .bind(&capabilities)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists(format!("Agent {} already exists", new_agent.agent_id))
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        Ok(agent.into())
    }

    pub async fn find_by_id(&self, agent_id: &str) -> DbResult<Option<Agent>> {
        let agent = sqlx::query_as::<_, AgentRow>(
            r#"
            SELECT
                id, agent_id, agent_secret_hash, agent_name, org_id,
                capabilities, roles, identity_id, api_key_id,
                agent_token_hash, agent_token_expires_at, status,
                agent_description, last_used_at, created_at, updated_at
            FROM agents WHERE agent_id = $1
            "#,
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(agent.map(|a| a.into()))
    }

    pub async fn find_by_uuid(&self, id: Uuid) -> DbResult<Option<Agent>> {
        self.find_by_id(&id.to_string()).await
    }

    pub async fn find_by_org(&self, org_id: Uuid) -> DbResult<Vec<Agent>> {
        let agents = sqlx::query_as::<_, AgentRow>(
            r#"
            SELECT
                id, agent_id, agent_secret_hash, agent_name, org_id,
                capabilities, roles, identity_id, api_key_id,
                agent_token_hash, agent_token_expires_at, status,
                agent_description, last_used_at, created_at, updated_at
            FROM agents WHERE org_id = $1
            "#,
        )
        .bind(org_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(agents.into_iter().map(|a| a.into()).collect())
    }

    pub async fn verify_secret(&self, agent_id: &str, secret: &str) -> DbResult<bool> {
        let agent = self.find_by_id(agent_id).await?;
        match agent {
            Some(a) => {
                if a.agent_secret_hash.is_empty() {
                    return Ok(false);
                }
                let valid = bcrypt::verify(secret, &a.agent_secret_hash).map_err(|e| {
                    DbError::ValidationError(format!("Failed to verify secret: {}", e))
                })?;
                Ok(valid)
            }
            None => Ok(false),
        }
    }

    pub async fn update_roles(&self, agent_id: &str, roles: Vec<String>) -> DbResult<()> {
        sqlx::query("UPDATE agents SET roles = $1, updated_at = NOW() WHERE agent_id = $2")
            .bind(&roles)
            .bind(agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(())
    }

    pub async fn update_org(&self, agent_id: &str, org_id: Option<Uuid>) -> DbResult<()> {
        sqlx::query("UPDATE agents SET org_id = $1, updated_at = NOW() WHERE agent_id = $2")
            .bind(org_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(())
    }

    pub async fn update_capabilities(
        &self,
        agent_id: &str,
        capabilities: Vec<String>,
    ) -> DbResult<()> {
        sqlx::query("UPDATE agents SET capabilities = $1, updated_at = NOW() WHERE agent_id = $2")
            .bind(&capabilities)
            .bind(agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct AgentRow {
    id: Uuid,
    agent_id: String,
    agent_secret_hash: String,
    agent_name: Option<String>,
    org_id: Option<Uuid>,
    capabilities: serde_json::Value,
    roles: Vec<String>,
    identity_id: Option<Uuid>,
    api_key_id: Option<Uuid>,
    agent_token_hash: Option<String>,
    agent_token_expires_at: Option<DateTime<Utc>>,
    status: Option<String>,
    agent_description: Option<String>,
    last_used_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<AgentRow> for Agent {
    fn from(row: AgentRow) -> Self {
        let capabilities: Vec<String> = row
            .capabilities
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            id: row.id,
            agent_id: row.agent_id,
            agent_secret_hash: row.agent_secret_hash,
            agent_name: row.agent_name,
            org_id: row.org_id,
            capabilities,
            roles: row.roles,
            identity_id: row.identity_id,
            api_key_id: row.api_key_id,
            agent_token_hash: row.agent_token_hash,
            agent_token_expires_at: row.agent_token_expires_at,
            status: row.status.unwrap_or_else(|| "active".to_string()),
            agent_description: row.agent_description,
            last_used_at: row.last_used_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
