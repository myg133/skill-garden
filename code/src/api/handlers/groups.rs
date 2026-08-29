//! 组管理 handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{require_admin, ApiState};

pub async fn list_groups_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListGroupsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let org_id = query.organization_id;
    let groups = if let Some(org_id) = org_id {
        state
            .group
            .list_by_organization(org_id)
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    } else {
        state
            .group
            .list()
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?
    };
    Ok((StatusCode::OK, Json(serde_json::json!({ "data": groups }))))
}

pub async fn create_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::CreateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let subject = agent_context.subject;
    let permission_overrides = body.permission_overrides.clone();
    let new_group: crate::models::group::NewGroup = body.into();
    let group = state
        .group
        .create(new_group)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if let Some(overrides) = permission_overrides {
        let creator_id = uuid::Uuid::parse_str(&subject).ok();
        for ov in overrides {
            state
                .group_perm_override_repo
                .upsert_override(
                    crate::models::group_permission_override::NewGroupPermissionOverride {
                        group_id: group.id,
                        role_name: ov.role_name,
                        permission_code: ov.permission_code,
                        granted: ov.granted,
                        created_by: creator_id,
                    },
                )
                .await
                .map_err(|e| ApiError::BadRequest(e.to_string()))?;
        }
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_created".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group.id.to_string()),
            details: serde_json::json!({
                "group_name": group.name,
                "organization_id": group.organization_id,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(group).unwrap()),
    ))
}

pub async fn get_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let group = state
        .group
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Group not found".to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn update_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateGroupBody>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    let group = state
        .group
        .update(id, body.into())
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::to_value(group).unwrap())))
}

pub async fn delete_group_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    require_admin(&state, &agent_context).await?;
    state
        .group
        .delete(id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    Ok((StatusCode::OK, Json(serde_json::json!({"deleted": id}))))
}