//! 技能分组关联 handlers

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_org_member, ApiState};

pub async fn list_skill_groups_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let repo = GroupSkillRepository::new(pool);

    let associations = repo
        .list_by_skill(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let responses: Vec<crate::api::models::SkillGroupResponse> = associations
        .into_iter()
        .map(|a| crate::api::models::SkillGroupResponse {
            skill_id: a.skill_id,
            group_id: a.group_id,
            group_name: String::new(),
            added_at: a.added_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::to_value(responses).unwrap()),
    ))
}

pub async fn add_skill_to_group_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::AddSkillToGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    use crate::db::repositories::group_skill::GroupSkillRepository;
    use crate::models::group_skill::NewGroupSkill;
    let pool = state.agent_repo.pool().clone();
    let group_repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = group_repo
        .find_by_id(body.group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", body.group_id)))?;
    require_org_member(&state, &agent_context, group.organization_id, None).await?;

    let repo = GroupSkillRepository::new(pool);
    repo.associate_skill(NewGroupSkill {
        group_id: body.group_id,
        skill_id: skill_id.clone(),
        added_by: None,
    })
    .await
    .map_err(|e| ApiError::BadRequest(format!("Failed to add skill to group: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "skill_added_to_group".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"group_id": body.group_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "message": "Skill added to group",
            "skill_id": skill_id,
            "group_id": body.group_id,
        })),
    ))
}

pub async fn remove_skill_from_group_handler(
    State(state): State<ApiState>,
    Path((skill_id, group_id)): Path<(String, Uuid)>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::group::GroupRepository;
    use crate::db::repositories::group_skill::GroupSkillRepository;
    let pool = state.agent_repo.pool().clone();
    let group_repo = GroupRepository::new(pool.clone());

    // Verify group exists and check org membership
    let group = group_repo
        .find_by_id(group_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Group {} not found", group_id)))?;
    require_org_member(&state, &agent_context, group.organization_id, None).await?;

    let repo = GroupSkillRepository::new(pool);
    repo.dissociate_skill(group_id, &skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to remove skill from group: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "skill_removed_from_group".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"group_id": group_id}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill removed from group",
            "skill_id": skill_id,
            "group_id": group_id,
        })),
    ))
}


