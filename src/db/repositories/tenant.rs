//! Tenant Repository

use std::collections::HashMap;

use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::error::{DbError, DbResult};
use crate::models::tenant::{
    NewTenant, RequestStatus, Tenant, TenantCreationRequest, TenantStatus, TenantUpdate,
};

#[derive(Clone)]
pub struct TenantRepository {
    pool: PgPool,
}

impl TenantRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, new_tenant: NewTenant) -> DbResult<Tenant> {
        let billing_plan = new_tenant
            .billing_plan
            .unwrap_or_else(|| "free".to_string());

        let tenant = sqlx::query_as::<_, TenantRow>(
            r#"INSERT INTO tenants (name, slug, billing_plan, sso_config, settings) VALUES ($1, $2, $3, $4, $5) RETURNING id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at"#,
        )
        .bind(&new_tenant.name)
        .bind(&new_tenant.slug)
        .bind(&billing_plan)
        .bind(&new_tenant.sso_config)
        .bind(&new_tenant.settings)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists(format!("Tenant with slug '{}' already exists", new_tenant.slug))
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        Ok(tenant.into())
    }

    pub async fn find_by_id(&self, id: Uuid) -> DbResult<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, TenantRow>(
            r#"SELECT id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at FROM tenants WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tenant.map(|t| t.into()))
    }

    pub async fn find_by_slug(&self, slug: &str) -> DbResult<Option<Tenant>> {
        let tenant = sqlx::query_as::<_, TenantRow>(
            r#"SELECT id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at FROM tenants WHERE slug = $1"#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tenant.map(|t| t.into()))
    }

    /// 批量根据 ID 列表查询租户名称（避免 N+1 查询）
    pub async fn find_names_by_ids(&self, ids: &[Uuid]) -> DbResult<HashMap<Uuid, String>> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let rows =
            sqlx::query_as::<_, TenantNameRow>("SELECT id, name FROM tenants WHERE id = ANY($1)")
                .bind(ids)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| (r.id, r.name)).collect())
    }

    pub async fn list_all(&self, limit: i64, offset: i64) -> DbResult<Vec<Tenant>> {
        let tenants = sqlx::query_as::<_, TenantRow>(
            r#"SELECT id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at FROM tenants ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tenants.into_iter().map(|t| t.into()).collect())
    }

    pub async fn update(&self, id: Uuid, update: TenantUpdate) -> DbResult<Tenant> {
        let current = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| DbError::NotFound("Tenant not found".to_string()))?;

        let name = update.name.unwrap_or(current.name);
        let slug = update.slug.unwrap_or(current.slug);
        let status = update.status.unwrap_or(current.status);
        let billing_plan = update.billing_plan.or(current.billing_plan);
        let sso_config = update.sso_config.or(current.sso_config);
        let settings = update.settings.unwrap_or(current.settings);

        let tenant = sqlx::query_as::<_, TenantRow>(
            r#"UPDATE tenants SET name = $1, slug = $2, status = $3, billing_plan = $4, sso_config = $5, settings = $6, updated_at = NOW() WHERE id = $7 RETURNING id, name, slug, status, billing_plan, sso_config, settings, created_by, created_at, updated_at"#,
        )
        .bind(&name)
        .bind(&slug)
        .bind(status.to_string())
        .bind(&billing_plan)
        .bind(&sso_config)
        .bind(&settings)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(tenant.into())
    }

    pub async fn delete(&self, id: Uuid) -> DbResult<()> {
        sqlx::query("DELETE FROM tenants WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::QueryError(e.to_string()))?;
        Ok(())
    }

    // ===== Tenant Creation Request methods =====

    pub async fn create_tenant_request(
        &self,
        request: TenantCreationRequest,
    ) -> DbResult<TenantCreationRequest> {
        let row = sqlx::query_as::<_, TenantCreationRequestRow>(
            r#"INSERT INTO tenant_creation_requests (id, applicant_id, applicant_name, applicant_email, tenant_name, tenant_slug, message, status, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING id, applicant_id, applicant_name, applicant_email, tenant_name, tenant_slug, message, status, reviewed_by, reviewed_at, review_note, tenant_id, created_at, updated_at"#,
        )
        .bind(&request.id)
        .bind(&request.applicant_id)
        .bind(&request.applicant_name)
        .bind(&request.applicant_email)
        .bind(&request.tenant_name)
        .bind(&request.tenant_slug)
        .bind(&request.message)
        .bind(request.status.to_string())
        .bind(&request.created_at)
        .bind(&request.updated_at)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate key") {
                DbError::AlreadyExists("Request already exists".to_string())
            } else {
                DbError::QueryError(e.to_string())
            }
        })?;

        Ok(row.into())
    }

    pub async fn get_tenant_request(&self, id: Uuid) -> DbResult<Option<TenantCreationRequest>> {
        let row = sqlx::query_as::<_, TenantCreationRequestRow>(
            r#"SELECT id, applicant_id, applicant_name, applicant_email, tenant_name, tenant_slug, message, status, reviewed_by, reviewed_at, review_note, tenant_id, created_at, updated_at FROM tenant_creation_requests WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    pub async fn list_tenant_requests(
        &self,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<TenantCreationRequest>> {
        let rows = sqlx::query_as::<_, TenantCreationRequestRow>(
            r#"SELECT id, applicant_id, applicant_name, applicant_email, tenant_name, tenant_slug, message, status, reviewed_by, reviewed_at, review_note, tenant_id, created_at, updated_at FROM tenant_creation_requests ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn list_pending_tenant_requests(
        &self,
        limit: i64,
        offset: i64,
    ) -> DbResult<Vec<TenantCreationRequest>> {
        let rows = sqlx::query_as::<_, TenantCreationRequestRow>(
            r#"SELECT id, applicant_id, applicant_name, applicant_email, tenant_name, tenant_slug, message, status, reviewed_by, reviewed_at, review_note, tenant_id, created_at, updated_at FROM tenant_creation_requests WHERE status = 'pending' ORDER BY created_at DESC LIMIT $1 OFFSET $2"#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn count_tenant_requests_by_applicant(&self, applicant_id: Uuid) -> DbResult<i64> {
        let row = sqlx::query_as::<_, (i64,)>(
            r#"SELECT COUNT(*) as count FROM tenant_creation_requests WHERE applicant_id = $1"#,
        )
        .bind(applicant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.0)
    }

    pub async fn update_tenant_request_status(
        &self,
        id: Uuid,
        status: RequestStatus,
        reviewed_by: Uuid,
        review_note: Option<String>,
        tenant_id: Option<Uuid>,
    ) -> DbResult<TenantCreationRequest> {
        let row = sqlx::query_as::<_, TenantCreationRequestRow>(
            r#"UPDATE tenant_creation_requests SET status = $1, reviewed_by = $2, reviewed_at = NOW(), review_note = $3, tenant_id = $4, updated_at = NOW() WHERE id = $5 RETURNING id, applicant_id, applicant_name, applicant_email, tenant_name, tenant_slug, message, status, reviewed_by, reviewed_at, review_note, tenant_id, created_at, updated_at"#,
        )
        .bind(status.to_string())
        .bind(reviewed_by)
        .bind(&review_note)
        .bind(tenant_id)
        .bind(id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.into())
    }

    pub async fn check_pending_request_exists(&self, applicant_id: Uuid) -> DbResult<bool> {
        let row = sqlx::query_as::<_, (bool,)>(
            r#"SELECT EXISTS(SELECT 1 FROM tenant_creation_requests WHERE applicant_id = $1 AND status = 'pending') as exists"#,
        )
        .bind(applicant_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DbError::QueryError(e.to_string()))?;

        Ok(row.0)
    }
}

#[derive(sqlx::FromRow)]
struct TenantRow {
    id: Uuid,
    name: String,
    slug: String,
    status: String,
    billing_plan: Option<String>,
    sso_config: Option<serde_json::Value>,
    settings: serde_json::Value,
    created_by: Option<Uuid>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct TenantNameRow {
    id: Uuid,
    name: String,
}

#[derive(sqlx::FromRow, Debug)]
struct TenantCreationRequestRow {
    id: Uuid,
    applicant_id: Uuid,
    applicant_name: String,
    applicant_email: String,
    tenant_name: String,
    tenant_slug: String,
    message: Option<String>,
    status: String,
    reviewed_by: Option<Uuid>,
    reviewed_at: Option<chrono::DateTime<Utc>>,
    review_note: Option<String>,
    tenant_id: Option<Uuid>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl From<TenantCreationRequestRow> for TenantCreationRequest {
    fn from(row: TenantCreationRequestRow) -> Self {
        Self {
            id: row.id,
            applicant_id: row.applicant_id,
            applicant_name: row.applicant_name,
            applicant_email: row.applicant_email,
            tenant_name: row.tenant_name,
            tenant_slug: row.tenant_slug,
            message: row.message,
            status: RequestStatus::from(row.status.as_str()),
            reviewed_by: row.reviewed_by,
            reviewed_at: row.reviewed_at,
            review_note: row.review_note,
            tenant_id: row.tenant_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

impl From<TenantRow> for Tenant {
    fn from(row: TenantRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            slug: row.slug,
            status: TenantStatus::from(row.status.as_str()),
            billing_plan: row.billing_plan,
            sso_config: row.sso_config,
            settings: row.settings,
            created_by: row.created_by,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}
