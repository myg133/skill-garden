//! 公共辅助函数、类型别名和权限检查

use std::sync::Arc;

use crate::api::error::ApiError;
use crate::api::http_state::AppRouterState;
use crate::api::jwt::AgentContext;

pub type ApiState = Arc<AppRouterState>;

/// 辅助函数：从 Skill 模型中提取字段并执行权限校验
pub(crate) async fn check_skill_perm(
    state: &ApiState,
    identity_id: Option<uuid::Uuid>,
    skill: &crate::models::Skill,
    action: crate::services::SkillAction,
) -> Result<(), ApiError> {
    let vis_str = match &skill.visibility {
        crate::models::skill_policy::Visibility::Private => "private",
        crate::models::skill_policy::Visibility::GroupVisible => "group_visible",
        crate::models::skill_policy::Visibility::OrgVisible => "org_visible",
        crate::models::skill_policy::Visibility::TenantVisible => "tenant_visible",
        crate::models::skill_policy::Visibility::Marketplace => "marketplace",
        crate::models::skill_policy::Visibility::Shared => "shared",
    };
    check_skill_perm_raw(
        state,
        identity_id,
        &skill.owner_type,
        skill.owner_id,
        skill.author_identity_id,
        &skill.status,
        vis_str,
        skill.marketplace_status.as_deref(),
        action,
    )
    .await
}

/// 辅助函数：使用原始字段值执行权限校验
pub(crate) async fn check_skill_perm_db(
    state: &ApiState,
    identity_id: Option<uuid::Uuid>,
    skill: &crate::db::repositories::skill::Skill,
    action: crate::services::SkillAction,
) -> Result<(), ApiError> {
    check_skill_perm_raw(
        state,
        identity_id,
        &skill.owner_type,
        skill.owner_id,
        skill.author_identity_id,
        &skill.status,
        &skill.visibility,
        skill.marketplace_status.as_deref(),
        action,
    )
    .await
}

/// 辅助函数：使用原始字段值执行权限校验
pub(crate) async fn check_skill_perm_raw(
    state: &ApiState,
    identity_id: Option<uuid::Uuid>,
    owner_type: &str,
    owner_id: Option<uuid::Uuid>,
    author_identity_id: Option<uuid::Uuid>,
    skill_status: &str,
    visibility: &str,
    marketplace_status: Option<&str>,
    action: crate::services::SkillAction,
) -> Result<(), ApiError> {
    let id_id = identity_id.ok_or_else(|| {
        tracing::warn!(
            "Permission check blocked: no identity_id in token (action={:?}, owner_type={})",
            action,
            owner_type
        );
        ApiError::Forbidden("身份信息缺失，请使用新版 API Key 重新认证".to_string())
    })?;

    state
        .permission
        .check_skill_permission(
            id_id,
            owner_type,
            owner_id,
            author_identity_id,
            skill_status,
            visibility,
            marketplace_status,
            action,
        )
        .await
        .map_err(|e| ApiError::Forbidden(e))?;
    Ok(())
}

/// 辅助函数：批量解析 nt_roles 中的租户中的租户名称（避免 ）循环查询）
pub(crate) async fn build_tenant_role_infos(
    state: &ApiState,
    tenant_roles: &[(uuid::Uuid, String)],
) -> Vec<crate::api::models::TenantRoleInfo> {
    if tenant_roles.is_empty() {
        return Vec::new();
    }
    let ids: Vec<uuid::Uuid> = tenant_roles.iter().map(|(id, _)| *id).collect();
    let name_map = state
        .tenant
        .get_names_by_ids(&ids)
        .await
        .unwrap_or_default();

    tenant_roles
        .iter()
        .map(
            |(tenant_id, role_name)| crate::api::models::TenantRoleInfo {
                tenant_id: *tenant_id,
                tenant_name: name_map
                    .get(tenant_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string()),
                role_name: role_name.clone(),
            },
        )
        .collect()
}

/// 统一的管理员权限检查
pub(crate) async fn require_admin(
    state: &ApiState,
    agent_context: &AgentContext,
) -> Result<uuid::Uuid, ApiError> {
    if agent_context.roles.iter().any(|r| r == "admin") {
        return agent_context.require_identity();
    }
    let identity_id = agent_context.require_identity()?;
    let is_admin = state
        .permission
        .is_any_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if is_admin {
        return Ok(identity_id);
    }
    Err(ApiError::Forbidden("Admin access required".to_string()))
}

/// 市场管理员权限检查（super_admin / marketplace_admin / marketplace_reviewer）
pub(crate) async fn require_marketplace_admin(
    state: &ApiState,
    agent_context: &AgentContext,
) -> Result<uuid::Uuid, ApiError> {
    if agent_context.roles.iter().any(|r| r == "admin") {
        return agent_context.require_identity();
    }
    let identity_id = agent_context.require_identity()?;
    let has_role = state
        .permission
        .has_any_system_role(
            identity_id,
            &["super_admin", "marketplace_admin", "marketplace_reviewer"],
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if has_role {
        return Ok(identity_id);
    }
    Err(ApiError::Forbidden(
        "Marketplace admin access required".to_string(),
    ))
}

/// 市场管理员权限检查（仅 super_admin / marketplace_admin，不含 marketplace_reviewer）
pub(crate) async fn require_marketplace_admin_only(
    state: &ApiState,
    agent_context: &AgentContext,
) -> Result<uuid::Uuid, ApiError> {
    if agent_context.roles.iter().any(|r| r == "admin") {
        return agent_context.require_identity();
    }
    let identity_id = agent_context.require_identity()?;
    let has_role = state
        .permission
        .has_any_system_role(identity_id, &["super_admin", "marketplace_admin"])
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if has_role {
        return Ok(identity_id);
    }
    Err(ApiError::Forbidden(
        "Marketplace admin (full) access required".to_string(),
    ))
}

/// 基本身份验证检查（任何已登录用户）
pub(crate) async fn require_auth(agent_context: &AgentContext) -> Result<uuid::Uuid, ApiError> {
    agent_context.require_identity()
}

/// 验证当前用户是指定组织的成员
pub(crate) async fn require_org_member(
    state: &ApiState,
    agent_context: &AgentContext,
    org_id: uuid::Uuid,
    min_role: Option<crate::models::org_membership::OrgRole>,
) -> Result<uuid::Uuid, ApiError> {
    let identity_id = agent_context.require_identity()?;
    let is_super = state
        .permission
        .is_super_admin(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if is_super {
        return Ok(identity_id);
    }
    let tenant_ids = state
        .permission
        .get_tenant_admin_tenant_ids(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if !tenant_ids.is_empty() {
        use crate::db::repositories::organization::OrganizationRepository;
        let pool = state.agent_repo.pool().clone();
        let org_repo = OrganizationRepository::new(pool);
        if let Ok(Some(org)) = org_repo.find_by_id(org_id).await {
            if let Some(tid) = org.tenant_id {
                if tenant_ids.contains(&tid) {
                    return Ok(identity_id);
                }
            }
        }
    }
    let ctx = state
        .permission
        .build_context(identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    let role_str = ctx
        .org_roles
        .iter()
        .find(|(id, _)| *id == org_id)
        .map(|(_, role)| role.clone())
        .ok_or_else(|| ApiError::Forbidden("Not a member of this organization".to_string()))?;
    let role = crate::models::org_membership::OrgRole::from(role_str.as_str());
    if let Some(min_role) = min_role {
        if role < min_role {
            return Err(ApiError::Forbidden(format!(
                "Requires at least {} role in this organization",
                min_role
            )));
        }
    }
    Ok(identity_id)
}

/// 通过 slug 或 UUID 解析组织 ID
pub(crate) async fn resolve_org_id(
    pool: &sqlx::PgPool,
    slug_or_id: &str,
) -> Result<(uuid::Uuid, String), ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let org_repo = OrganizationRepository::new(pool.clone());
    if let Ok(Some(org)) = org_repo.find_by_slug(slug_or_id).await {
        let slug = org.slug.unwrap_or_else(|| org.id.to_string());
        return Ok((org.id, slug));
    }
    if let Ok(id) = uuid::Uuid::parse_str(slug_or_id) {
        if let Ok(Some(org)) = org_repo.find_by_id(id).await {
            let slug = org.slug.unwrap_or_else(|| org.id.to_string());
            return Ok((org.id, slug));
        }
    }
    Err(ApiError::NotFound(format!(
        "Organization '{}' not found",
        slug_or_id
    )))
}
