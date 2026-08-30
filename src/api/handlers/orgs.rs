//! 组织管理 handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, require_org_member, ApiState};

/// Organization handlers

/// 验证当前用户是指定组织的成员，可选最低角色要求
/// super_admin 全局通过；tenant_admin 对其租户下所有组织通过。
/// 非管理员用户回退到 build_context 的 org_roles 检查。
pub async fn create_org_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let org = state
        .organization
        .create_org(
            body.name,
            body.slug,
            body.display_name,
            body.description,
            body.tenant_id,
        )
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(org).unwrap()),
    ))
}

pub async fn get_org_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state
        .organization
        .get_org(org_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn list_orgs_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListOrgsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let orgs = if let Some(tenant_id) = query.tenant_id {
        state
            .organization
            .list_orgs_by_tenant(tenant_id, limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .organization
            .list_orgs(limit, offset)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": orgs }))))
}

pub async fn update_org_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_org_member(
        &state,
        &agent_context,
        org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let org = state
        .organization
        .update_org(org_id, body.name, body.display_name, body.description)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn delete_org_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    state
        .organization
        .delete_org(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": org_id}))))
}

/// Organization member handlers

pub async fn list_org_members_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    let pool = state.agent_repo.pool().clone();

    require_admin(&state, &agent_context).await?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo
        .list_all_members(org_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn add_org_member_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AddOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    use crate::db::repositories::agent::NewAgent;

    let secret = uuid::Uuid::new_v4().to_string();

    let new_agent = NewAgent {
        agent_id: body.agent_id.clone(),
        agent_secret: secret,
        agent_name: body.name.clone(),
        org_id: Some(org_id),
        capabilities: Some(Vec::<String>::new()),
    };

    state
        .agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Member added successfully",
            "agent_id": body.agent_id
        })),
    ))
}

pub async fn remove_org_member_handler(
    State(state): State<ApiState>,
    Path((_org_id, subject)): Path<(Uuid, String)>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    state
        .agent_repo
        .update_org(&subject, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"removed": subject})),
    ))
}

pub async fn get_org_stats_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;

    let pool = state.agent_repo.pool();

    let members_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agents WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let skills_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE author_subject IN (SELECT subject FROM agents WHERE org_id = $1)"
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let sessions_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sessions WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let tools_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM org_tools WHERE org_id = $1")
            .bind(org_id)
            .fetch_one(pool)
            .await
            .unwrap_or(0);

    let response = crate::api::models::OrgStatsResponse {
        org_id,
        members_count,
        skills_count,
        sessions_count,
        tools_count,
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_org_by_slug_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = OrganizationRepository::new(pool);

    let org = repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn create_org_skill_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateOrgSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let membership = org_membership_repo
        .get_member(identity_id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::Forbidden("Not a member of this organization".to_string()))?;

    let owner_type = body
        .owner_type
        .unwrap_or_else(|| "organization".to_string());

    if owner_type == "organization" {
        let role_str = membership.role.to_string();
        if role_str != "owner" && role_str != "admin" && role_str != "developer" {
            return Err(ApiError::Unauthorized(
                "Need developer role to create org skills".to_string(),
            ));
        }
    }

    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
        _ => crate::models::skill_policy::Visibility::OrgVisible,
    });

    let new_skill = crate::models::NewSkill {
        name: body.name,
        description: body.description,
        tags: body.tags,
        content: body.content,
        version: body.version.unwrap_or_else(|| "1.0.0".to_string()),
        git_url: body.git_url.clone(),
        visibility,
        tools: body.tools.clone(),
        owner_type,
        owner_id: Some(org.id),
        author_identity_id: Some(identity_id),
    };

    let skill = state
        .registry
        .create_skill(new_skill, &subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create skill: {}", e)))?;

    let response = crate::api::models::SkillCreatedResponse {
        message: "Skill created successfully".to_string(),
        skill_id: skill.id,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn invite_org_member_handler(
    State(state): State<ApiState>,
    Path(slug): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::InviteOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_identity = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email '{}' not found", body.email)))?;

    let inviter_id = agent_context.require_identity()?;

    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    org_membership_repo
        .add_member(
            target_identity.id,
            org.id,
            body.role.as_str().into(),
            Some(inviter_id),
        )
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": format!("{} added to {}", body.email, slug),
            "organization_id": org.id,
            "identity_id": target_identity.id,
            "role": body.role,
        })),
    ))
}

pub async fn invite_org_member_by_id_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<uuid::Uuid>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::InviteOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_identity = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User with email '{}' not found", body.email)))?;

    let inviter_id = agent_context.require_identity()?;

    let org_membership_repo = OrgMembershipRepository::new(pool.clone());
    org_membership_repo
        .add_member(
            target_identity.id,
            org.id,
            body.role.as_str().into(),
            Some(inviter_id),
        )
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add member: {}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": format!("{} added to organization {}", body.email, org_id),
            "organization_id": org.id,
            "identity_id": target_identity.id,
            "role": body.role,
        })),
    ))
}

pub async fn update_org_member_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((slug, username)): Path<(String, String)>,
    Json(body): Json<crate::api::models::UpdateOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .update_role(identity.id, org.id, body.role.as_str().into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update role: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} role updated in {}", username, slug),
            "role": body.role,
        })),
    ))
}

pub async fn remove_org_member_by_slug_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((slug, username)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .remove_member(identity.id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} removed from {}", username, slug),
        })),
    ))
}

pub async fn update_org_member_by_id_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((org_id, username)): Path<(uuid::Uuid, String)>,
    Json(body): Json<crate::api::models::UpdateOrgMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .update_role(identity.id, org.id, body.role.as_str().into())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update role: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} role updated in {}", username, org_id),
            "role": body.role,
        })),
    ))
}

pub async fn remove_org_member_by_id_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((org_id, username)): Path<(uuid::Uuid, String)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    require_org_member(
        &state,
        &agent_context,
        org.id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let identity = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    org_membership_repo
        .remove_member(identity.id, org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove member: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("{} removed from {}", username, org_id),
        })),
    ))
}

pub async fn list_org_members_by_slug_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(&state, &agent_context, org.id, None).await?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo
        .list_all_members(org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list members: {}", e)))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn list_org_members_by_id_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::org_membership::OrgMembershipRepository;
    use crate::db::repositories::organization::OrganizationRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_id(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", org_id)))?;

    require_org_member(&state, &agent_context, org.id, None).await?;

    let org_membership_repo = OrgMembershipRepository::new(pool);
    let members = org_membership_repo
        .list_all_members(org.id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list members: {}", e)))?;

    Ok((StatusCode::OK, Json(members)))
}

pub async fn list_org_skills_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(&state, &agent_context, org.id, None).await?;

    let skill_repo = SkillRepository::new(pool);
    let skills = skill_repo
        .list_by_org(&org.id.to_string())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list skills: {}", e)))?;

    Ok((StatusCode::OK, Json(skills)))
}

pub async fn list_org_reviews_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::organization::OrganizationRepository;
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();

    let org_repo = OrganizationRepository::new(pool.clone());
    let org = org_repo
        .find_by_slug(&slug)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Organization '{}' not found", slug)))?;

    require_org_member(&state, &agent_context, org.id, None).await?;

    let skill_repo = SkillRepository::new(pool);
    let skills = skill_repo
        .list_by_org(&org.id.to_string())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list skills: {}", e)))?;

    let in_review: Vec<_> = skills
        .into_iter()
        .filter(|s| s.status == "pending_review")
        .collect();

    Ok((StatusCode::OK, Json(in_review)))
}

// ========================
// Organization Join Request Handlers
// ========================

/// Create a join request for an organization
/// POST /orgs/:id/join-request
pub async fn create_join_request_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<Uuid>,
    Json(body): Json<CreateJoinRequestBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()
        .map_err(|e| ApiError::Unauthorized(e.to_string()))?;

    // Check if organization exists and join policy allows requests
    let org = state.organization.get_org(org_id).await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;

    // Check join policy
    match org.join_policy.as_deref() {
        Some("invite_only") => {
            return Err(ApiError::Forbidden("This organization does not accept join requests".to_string()));
        }
        Some("open") => {
            // For open organizations, auto-approve by creating membership directly
            return Err(ApiError::BadRequest("This organization is open. No request needed, use invite API instead.".to_string()));
        }
        _ => { /* approval_required is the default, continue */ }
    }

    // Check if user is already a member
    let is_member = state.permission.is_org_member(identity_id, org_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if is_member {
        return Err(ApiError::BadRequest("You are already a member of this organization".to_string()));
    }

    // Check if there's already a pending request
    let has_pending = state.org_join_request.has_pending_request(org_id, identity_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if has_pending {
        return Err(ApiError::BadRequest("You already have a pending join request for this organization".to_string()));
    }

    let request = state.org_join_request.create(org_id, identity_id, body.message).await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::json!({
        "id": request.id,
        "organization_id": request.organization_id,
        "status": request.status,
        "message": request.message,
        "created_at": request.created_at
    }))))
}

#[derive(serde::Deserialize)]
pub struct CreateJoinRequestBody {
    pub message: Option<String>,
}

/// Delete (cancel) a pending join request
/// DELETE /orgs/:id/join-request
pub async fn cancel_join_request_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()
        .map_err(|e| ApiError::Unauthorized(e.to_string()))?;

    state.org_join_request.cancel(org_id, identity_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": true}))))
}

/// Get user's pending request for an organization
/// GET /orgs/:id/my-join-request
pub async fn get_my_join_request_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()
        .map_err(|e| ApiError::Unauthorized(e.to_string()))?;

    // Try to find pending request
    let requests = state.org_join_request.list_by_org(org_id, Some("pending"), 1, 0).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let my_request = requests.into_iter().find(|r| r.identity_id == identity_id);

    match my_request {
        Some(req) => Ok((StatusCode::OK, Json(serde_json::json!({
            "id": req.id,
            "organization_id": req.organization_id,
            "status": req.status,
            "message": req.message,
            "created_at": req.created_at
        })))),
        None => Err(ApiError::NotFound("No pending join request found".to_string()))
    }
}

/// List join requests for an organization (admin only)
/// GET /orgs/:id/join-requests
pub async fn list_join_requests_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(org_id): Path<Uuid>,
    Query(query): Query<ListJoinRequestsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // Require org admin or higher
    require_org_member(
        &state,
        &agent_context,
        org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let status = query.status.as_deref();
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let requests = state.org_join_request.list_by_org(org_id, status, limit, offset).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let results: Vec<serde_json::Value> = requests.into_iter().map(|r| {
        serde_json::json!({
            "id": r.id,
            "organization_id": r.organization_id,
            "identity": {
                "id": r.identity.id,
                "name": r.identity.name,
                "email": r.identity.email,
                "username": r.identity.username
            },
            "status": r.status,
            "message": r.message,
            "reviewed_by": r.reviewed_by,
            "reviewed_at": r.reviewed_at,
            "created_at": r.created_at
        })
    }).collect();

    Ok((StatusCode::OK, Json(serde_json::json!({"data": results}))))
}

#[derive(serde::Deserialize)]
pub struct ListJoinRequestsQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Approve or reject a join request (admin only)
/// PUT /orgs/:id/join-requests/:request_id
pub async fn review_join_request_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path((org_id, request_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ReviewJoinRequestBody>,
) -> Result<impl IntoResponse, ApiError> {
    let reviewer_id = require_org_member(
        &state,
        &agent_context,
        org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    // Verify the request exists and belongs to this org
    let request = state.org_join_request.get(request_id).await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Join request not found".to_string()))?;

    if request.organization_id != org_id {
        return Err(ApiError::NotFound("Join request not found in this organization".to_string()));
    }

    if request.status != "pending" {
        return Err(ApiError::BadRequest("This request has already been processed".to_string()));
    }

    // Cannot review own request
    if request.identity_id == reviewer_id {
        return Err(ApiError::Forbidden("You cannot approve your own join request".to_string()));
    }

    let updated = match body.action.as_str() {
        "approve" => {
            // Update request status
            let updated = state.org_join_request.approve(request_id, reviewer_id).await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;

            // Auto-create org membership on approval
            use crate::db::repositories::org_membership::OrgMembershipRepository;
            let pool = state.agent_repo.pool().clone();
            let membership_repo = OrgMembershipRepository::new(pool);

            membership_repo.add_member(
                request.identity_id,
                org_id,
                crate::models::org_membership::OrgRole::Member,
                Some(reviewer_id),
            ).await.map_err(|e| ApiError::InternalError(e.to_string()))?;

            updated
        }
        "reject" => {
            state.org_join_request.reject(request_id, reviewer_id).await
                .map_err(|e| ApiError::InternalError(e.to_string()))?
        }
        _ => {
            return Err(ApiError::BadRequest("Invalid action. Use 'approve' or 'reject'".to_string()));
        }
    };

    Ok((StatusCode::OK, Json(serde_json::json!({
        "id": updated.id,
        "organization_id": updated.organization_id,
        "status": updated.status,
        "reviewed_by": updated.reviewed_by,
        "reviewed_at": updated.reviewed_at
    }))))
}

#[derive(serde::Deserialize)]
pub struct ReviewJoinRequestBody {
    pub action: String,
    pub message: Option<String>,
}




