//! 组织分组管理 handlers

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_org_member, resolve_org_id, ApiState};

// Org slug-based Group management (6.6)

pub async fn create_org_group_handler(
    State(state): State<ApiState>,
    Path(slug_or_id): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    require_org_member(
        &state,
        &agent_context,
        org_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let mut new_group: crate::models::group::NewGroup = body.into();
    new_group.organization_id = org_id;

    let group = state
        .group
        .create(new_group)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "group_created".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group.id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(group).unwrap()),
    ))
}

pub async fn list_org_groups_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(slug_or_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (org_id, _slug) = resolve_org_id(&pool, &slug_or_id).await?;

    require_org_member(&state, &agent_context, org_id, None).await?;

    let groups = state
        .group
        .list_by_organization(org_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}

pub async fn get_org_group_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (_org_id, _slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let group = state
        .group
        .get(group_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn update_org_group_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let group = state
        .group
        .update(group_id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn delete_org_group_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    state
        .group
        .delete(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_deleted".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"deleted": group_id})),
    ))
}

// Org slug/id-based Group member management (6.6)

pub async fn list_org_group_members_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, _slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let repo = GroupRepository::new(pool);
    let members = repo
        .list_members(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let member_info: Vec<crate::api::models::GroupMemberInfo> = members
        .into_iter()
        .map(|m| crate::api::models::GroupMemberInfo {
            agent_id: m.identity_id.to_string(),
            name: m.identity_name,
            email: m.email,
            username: m.username,
            role: m.role,
            joined_at: m.joined_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": member_info })),
    ))
}

pub async fn update_org_group_member_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id, username)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let target_id = uuid::Uuid::parse_str(&username)
        .map_err(|_| ApiError::BadRequest("Invalid member id".to_string()))?;

    let repo = GroupRepository::new(pool);
    repo.update_member_role(target_id, group_id, &body.role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update group member: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_role_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "member_id": username, "role": body.role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member role updated",
            "group_id": group_id,
            "member_id": username,
        })),
    ))
}

pub async fn remove_org_group_member_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id, username)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let target_id = uuid::Uuid::parse_str(&username)
        .map_err(|_| ApiError::BadRequest("Invalid member id".to_string()))?;

    let repo = GroupRepository::new(pool);
    repo.remove_member(target_id, group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_member_removed".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "member_id": username}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member removed",
            "group_id": group_id,
            "member_id": username,
        })),
    ))
}

// Org slug/id-based Group-Skill association (6.6)

pub async fn list_org_group_skills_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, _slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let repo = GroupSkillRepository::new(pool);
    let skills = repo
        .list_by_group(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": skills }))))
}

pub async fn add_org_group_skill_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id)): Path<(String, Uuid)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::AddSkillToGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let skill_id = body
        .skill_id
        .clone()
        .ok_or_else(|| ApiError::BadRequest("skill_id is required".to_string()))?;

    let repo = GroupSkillRepository::new(pool);
    repo.associate_skill(crate::models::group_skill::NewGroupSkill {
        group_id,
        skill_id: skill_id.clone(),
        added_by: None,
    })
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to associate skill: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_skill_associated".to_string(),
            resource_type: "group_skill".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "skill_id": skill_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Skill associated to group",
            "group_id": group_id,
            "skill_id": skill_id,
        })),
    ))
}

pub async fn remove_org_group_skill_handler(
    State(state): State<ApiState>,
    Path((slug_or_id, group_id, skill_id)): Path<(String, Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let (_org_id, slug) = resolve_org_id(&pool, &slug_or_id).await?;

    let repo = GroupSkillRepository::new(pool);
    repo.dissociate_skill(group_id, &skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to dissociate skill: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_skill_dissociated".to_string(),
            resource_type: "group_skill".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"org_slug": slug, "skill_id": skill_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill dissociated from group",
            "group_id": group_id,
            "skill_id": skill_id,
        })),
    ))
}


