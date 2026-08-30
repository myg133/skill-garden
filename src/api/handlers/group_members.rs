//! 组成员管理 handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use super::helpers::{require_org_member, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;

// Group member management handlers (6.6)

pub async fn list_group_members_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool);

    let members = repo
        .list_members(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list group members: {}", e)))?;

    let response: Vec<crate::api::models::GroupMemberInfo> = members
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

    Ok((StatusCode::OK, Json(response)))
}

pub async fn add_group_member_handler(
    State(state): State<ApiState>,
    Path(group_id): Path<Uuid>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AddGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = repo
        .find_by_id(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", group_id)))?;
    require_org_member(
        &state,
        &agent_context,
        group.organization_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_id = uuid::Uuid::parse_str(&body.agent_id)
        .map_err(|_| ApiError::BadRequest("Invalid subject".to_string()))?;

    let role = body.role.unwrap_or_else(|| "member".to_string());

    repo.add_member(target_id, group_id, &role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to add group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "group_member_added".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": body.agent_id, "role": role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Member added to group",
            "group_id": group_id,
            "member_id": body.agent_id,
        })),
    ))
}

pub async fn update_group_member_handler(
    State(state): State<ApiState>,
    Path((group_id, member_subject)): Path<(Uuid, String)>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupMemberBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = repo
        .find_by_id(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", group_id)))?;
    require_org_member(
        &state,
        &agent_context,
        group.organization_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_id = uuid::Uuid::parse_str(&member_subject)
        .map_err(|_| ApiError::BadRequest("Invalid member subject".to_string()))?;

    repo.add_member(target_id, group_id, &body.role)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "group_member_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": member_subject, "role": body.role}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member updated",
            "group_id": group_id,
            "member_id": member_subject,
        })),
    ))
}

pub async fn remove_group_member_handler(
    State(state): State<ApiState>,
    Path((group_id, member_subject)): Path<(Uuid, String)>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = repo
        .find_by_id(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", group_id)))?;
    require_org_member(
        &state,
        &agent_context,
        group.organization_id,
        Some(crate::models::org_membership::OrgRole::Admin),
    )
    .await?;

    let target_id = uuid::Uuid::parse_str(&member_subject)
        .map_err(|_| ApiError::BadRequest("Invalid member subject".to_string()))?;

    repo.remove_member(target_id, group_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove group member: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "group_member_removed".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({"member_id": member_subject}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group member removed",
            "group_id": group_id,
            "member_id": member_subject,
        })),
    ))
}
