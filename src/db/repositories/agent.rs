//! Agent repository

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use bcrypt::{hash, verify, DEFAULT_COST};
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: Uuid,
    pub agent_id: String,
    pub agent_secret_hash: String,
    pub agent_name: Option<String>,
    pub org_id: Option<Uuid>,
    pub capabilities: Vec<String>,
    pub roles: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

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

    pub async fn create(&self, new_agent: NewAgent) -> DbResult<Agent> {
        let secret_hash = hash(&new_agent.agent_secret, DEFAULT_COST)
            .map_err(|e| DbError::ValidationError(format!("Failed to hash secret: {}", e)))?;

        let roles = Vec::<String>::new();
        let capabilities = new_agent.capabilities.unwrap_or_default();

        let agent = sqlx::query_as::<_, AgentRow>(
            r#"
            INSERT INTO agents (agent_id, agent_secret_hash, agent_name, roles, capabilities)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, agent_id, agent_secret_hash, agent_name, org_id, capabilities, roles, created_at, updated_at
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
            SELECT id, agent_id, agent_secret_hash, agent_name, org_id, capabilities, roles, created_at, updated_at
            FROM agents WHERE agent_id = $1
            "#,
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(agent.map(|a| a.into()))
    }

    pub async fn find_by_org(&self, org_id: Uuid) -> DbResult<Vec<Agent>> {
        let agents = sqlx::query_as::<_, AgentRow>(
            r#"
            SELECT id, agent_id, agent_secret_hash, agent_name, org_id, capabilities, roles, created_at, updated_at
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
                let valid = verify(secret, &a.agent_secret_hash)
                    .map_err(|e| DbError::ValidationError(format!("Failed to verify secret: {}", e)))?;
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

    pub async fn update_org(&self, agent_id: &str, org_id: Uuid) -> DbResult<()> {
        sqlx::query("UPDATE agents SET org_id = $1, updated_at = NOW() WHERE agent_id = $2")
            .bind(org_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(())
    }

    pub async fn update_capabilities(&self, agent_id: &str, capabilities: Vec<String>) -> DbResult<()> {
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
    capabilities: Vec<String>,
    roles: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<AgentRow> for Agent {
    fn from(row: AgentRow) -> Self {
        Self {
            id: row.id,
            agent_id: row.agent_id,
            agent_secret_hash: row.agent_secret_hash,
            agent_name: row.agent_name,
            org_id: row.org_id,
            capabilities: row.capabilities,
            roles: row.roles,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
