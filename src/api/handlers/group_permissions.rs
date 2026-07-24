//! 分组权限管理 handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::ApiState;

// Group permission override handlers

pub async fn list_group_default_permissions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::role_permission::RolePermissionRepository;

    let pool = state.group_perm_override_repo.pool().clone();
    let role_perm_repo = RolePermissionRepository::new(pool);

    let lead_defaults = role_perm_repo
        .list_by_role("group", "lead")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let member_defaults = role_perm_repo
        .list_by_role("group", "member")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let to_codes = |perms: Vec<crate::models::role_permission::RolePermission>| -> Vec<String> {
        perms.into_iter().map(|p| p.permission_code).collect()
    };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "lead": to_codes(lead_defaults),
            "member": to_codes(member_defaults),
        })),
    ))
}

pub async fn list_group_permissions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(group_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::api::models::GroupPermissionInfo;
    use crate::db::repositories::role_permission::RolePermissionRepository;

    let pool = state.group_perm_override_repo.pool().clone();

    let role_perm_repo = RolePermissionRepository::new(pool.clone());

    let lead_defaults = role_perm_repo
        .list_by_role("group", "lead")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let member_defaults = role_perm_repo
        .list_by_role("group", "member")
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let overrides = state
        .group_perm_override_repo
        .list_by_group(group_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let is_overridden = |perm_code: &str| -> Option<bool> {
        overrides
            .iter()
            .find(|o| o.permission_code == perm_code)
            .map(|o| o.granted)
    };

    let to_info =
        |perms: Vec<crate::models::role_permission::RolePermission>| -> Vec<GroupPermissionInfo> {
            perms
                .into_iter()
                .map(|p| {
                    let code = p.permission_code;
                    let override_granted = is_overridden(&code);
                    GroupPermissionInfo {
                        permission_code: code,
                        granted: override_granted.unwrap_or(true),
                        is_default: override_granted.is_none(),
                    }
                })
                .collect()
        };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "lead": to_info(lead_defaults),
            "member": to_info(member_defaults),
        })),
    ))
}

pub async fn update_group_permission_handler(
    State(state): State<ApiState>,
    Path(group_id): Path<Uuid>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupPermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::models::group_permission_override::NewGroupPermissionOverride;

    let role_name = body.role_name.clone();
    let permission_code = body.permission_code.clone();

    let creator_id = uuid::Uuid::parse_str(&subject).ok();

    state
        .group_perm_override_repo
        .upsert_override(NewGroupPermissionOverride {
            group_id,
            role_name: body.role_name,
            permission_code: body.permission_code,
            granted: body.granted,
            created_by: creator_id,
        })
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_permission_updated".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({
                "role_name": role_name,
                "permission_code": permission_code,
                "granted": body.granted,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group permission override updated"
        })),
    ))
}

pub async fn delete_group_permission_handler(
    State(state): State<ApiState>,
    Path((group_id, permission_code)): Path<(Uuid, String)>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateGroupPermissionBody>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .group_perm_override_repo
        .delete_override(group_id, &body.role_name, &permission_code)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let role_name = body.role_name.clone();

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "group_permission_deleted".to_string(),
            resource_type: "group".to_string(),
            resource_id: Some(group_id.to_string()),
            details: serde_json::json!({
                "role_name": role_name,
                "permission_code": permission_code,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Group permission override deleted"
        })),
    ))
}

// --- Admin User Management Handlers (Feature #7) ---

pub async fn list_users_handler_admin(
    State(state): State<ApiState>,
    AgentContext { .. }: AgentContext,
    Query(query): Query<crate::api::models::ListUsersQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);
    let identity_type = query.identity_type.as_deref();

    let users = state
        .identity
        .list(limit, offset, identity_type)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::UserAdminResponse> = users
        .into_iter()
        .map(|u| crate::api::models::UserAdminResponse {
            id: u.id,
            identity_type: u.identity_type.to_string(),
            username: u.username,
            display_name: u.display_name,
            email: u.email,
            avatar_url: u.avatar_url,
            is_system_admin: u.is_system_admin,
            status: u.status.to_string(),
            created_at: u.created_at,
            updated_at: u.updated_at,
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": data,
            "total": data.len(),
            "limit": limit,
            "offset": offset,
        })),
    ))
}

pub async fn disable_user_handler_admin(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(user_id): Path<uuid::Uuid>,
    Json(body): Json<crate::api::models::DisableUserBody>,
) -> Result<impl IntoResponse, ApiError> {
    let status = if body.disabled { "disabled" } else { "active" };

    let update = crate::models::identity::IdentityUpdate {
        status: Some(status.into()),
        ..Default::default()
    };

    let updated = state
        .identity
        .update(user_id, update)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: if body.disabled {
                "user_disabled".to_string()
            } else {
                "user_enabled".to_string()
            },
            resource_type: "user".to_string(),
            resource_id: Some(user_id.to_string()),
            details: serde_json::json!({
                "username": updated.username,
                "status": status,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("User {} successfully", if body.disabled { "disabled" } else { "enabled" }),
            "user_id": user_id.to_string(),
        })),
    ))
}

pub async fn delete_user_handler_admin(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Path(user_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .identity
        .delete(user_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "user_deleted".to_string(),
            resource_type: "user".to_string(),
            resource_id: Some(user_id.to_string()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "User deleted successfully",
            "user_id": user_id.to_string(),
        })),
    ))
}

// --- Evaluation Query/Delete Handlers (Feature #8) ---

