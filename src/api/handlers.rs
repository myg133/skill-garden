//! API Route Handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::sync::Arc;

use crate::api::error::ApiError;
use crate::api::http_state::AppRouterState;
use crate::api::jwt::AgentContext;
use crate::models::{NewSkill, SkillUpdate};
use crate::models::evaluation::{ErrorType as EvalErrorType, EvalTag};
use crate::models::error::AppError;

pub type ApiState = Arc<AppRouterState>;

pub async fn health_handler(State(state): State<ApiState>) -> Result<impl IntoResponse, ApiError> {
    let skills_count = state.registry.count().await.map_err(|e| ApiError::InternalError(e.to_string()))?;
    let response = crate::api::models::HealthResponse {
        status: "OK".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        skills_count: skills_count as usize,
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn list_skills_handler(
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListSkillsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

    let skills = state.registry.list_skills().await.map_err(|e| ApiError::InternalError(e.to_string()))?;

    let mut filtered: Vec<_> = skills;

    if let Some(ref tag) = query.tag {
        filtered.retain(|s| s.tags.iter().any(|t| t == tag));
    }

    if let Some(ref keyword) = query.keyword {
        let keyword_lower = keyword.to_lowercase();
        filtered.retain(|s| {
            s.name.to_lowercase().contains(&keyword_lower)
                || s.description.to_lowercase().contains(&keyword_lower)
        });
    }

    let total = filtered.len();
    let start = (page - 1) * page_size;
    let end = (start + page_size).min(total);

    let page_items: Vec<_> = if start < total {
        filtered[start..end].to_vec()
    } else {
        vec![]
    };

    let response = crate::api::models::ListResponse::new(page_items, total, page, page_size);
    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state.registry.get_skill(&skill_id).await
        .map_err(|_| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    let stats = state.evaluator.get_stats(&skill_id).ok();

    let detail = crate::models::SkillDetail {
        metadata: (&skill).into(),
        content: skill.content,
        stats,
    };
    Ok((StatusCode::OK, Json(detail)))
}

pub async fn create_skill_handler(
    State(state): State<ApiState>,
    AgentContext { agent_id, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
        "shared" => crate::models::skill_policy::Visibility::Shared,
        _ => crate::models::skill_policy::Visibility::OrgVisible,
    });

    let new_skill = NewSkill {
        name: body.name,
        description: body.description,
        tags: body.tags,
        content: body.content,
        version: body.version.unwrap_or_else(|| "1.0.0".to_string()),
        git_url: body.git_url.clone(),
        visibility,
        tools: body.tools.clone(),
    };

    let skill = state.registry.create_skill(new_skill, &agent_id, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create skill: {}", e)))?;

    let response = crate::api::models::SkillCreatedResponse {
        message: "Skill created successfully".to_string(),
        skill_id: skill.id,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn update_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { agent_id, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "marketplace" => crate::models::skill_policy::Visibility::Marketplace,
        "shared" => crate::models::skill_policy::Visibility::Shared,
        _ => crate::models::skill_policy::Visibility::OrgVisible,
    });

    let update = SkillUpdate {
        description: body.description,
        tags: body.tags,
        content: body.content,
        git_url: body.git_url.clone(),
        visibility,
        tools: body.tools.clone(),
    };

    state.registry.update_skill(&skill_id, update, &agent_id, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update skill: {}", e)))?;

    let response = crate::api::models::MessageResponse {
        message: "Skill updated successfully".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn delete_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    state.registry.delete_skill(&skill_id, &state.search).await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delete skill: {}", e)))?;

    let response = crate::api::models::MessageResponse {
        message: "Skill deleted successfully".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_skill_stats_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state.evaluator.get_stats(&skill_id)
        .map_err(|_| ApiError::NotFound(format!("Skill {} not found or has no stats", skill_id)))?;

    Ok((StatusCode::OK, Json(stats)))
}

pub async fn create_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { agent_id, .. }: AgentContext,
    Json(body): Json<crate::api::models::CreateEvaluationBody>,
) -> Result<(StatusCode, Json<crate::api::models::EvaluationCreatedResponse>), ApiError> {
    let error_type = body.error_type.as_ref().and_then(|e| {
        match e.as_str() {
            "timeout" => Some(EvalErrorType::Timeout),
            "crash" => Some(EvalErrorType::Crash),
            "logic_error" => Some(EvalErrorType::LogicError),
            _ => Some(EvalErrorType::Other),
        }
    });

    let tags = body.tags.iter().filter_map(|t| {
        match t.as_str() {
            "reliable" => Some(EvalTag::Reliable),
            "fast" => Some(EvalTag::Fast),
            "stable" => Some(EvalTag::Stable),
            "experimental" => Some(EvalTag::Experimental),
            _ => None,
        }
    }).collect();

    let result = state.evaluator.add_evaluation(
        body.skill_id,
        agent_id,
        body.success,
        body.duration_ms,
        error_type,
        tags,
    )
    .await
    .map_err(|e: AppError| ApiError::BadRequest(e.to_string()))?;

    let response = crate::api::models::EvaluationCreatedResponse {
        message: "Evaluation recorded successfully".to_string(),
        evaluation_id: result.evaluation_id,
        new_stats: result.new_stats,
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn register_agent_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::RegisterAgentBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::agent::NewAgent;

    let secret = uuid::Uuid::new_v4().to_string();

    let new_agent = NewAgent {
        agent_id: body.agent_id.clone(),
        agent_secret: secret.clone(),
        agent_name: body.agent_name.clone(),
        org_id: None,
        capabilities: None,
    };

    state.agent_repo
        .create(new_agent)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to register agent: {}", e)))?;

    let response = crate::api::models::RegisterAgentResponse {
        agent_id: body.agent_id,
        secret,
        message: "Agent registered successfully. Store the secret securely - it will not be shown again.".to_string(),
    };
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn get_token_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::GetTokenBody>,
) -> Result<impl IntoResponse, ApiError> {
    let valid = state.agent_repo
        .verify_secret(&body.agent_id, &body.agent_secret)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Authentication error: {}", e)))?;

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    let token = crate::api::generate_token(&body.agent_id, vec![], vec![])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    let response = crate::api::models::TokenResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
    };
    Ok((StatusCode::OK, Json(response)))
}

/// Admin login handler for human administrators
pub async fn admin_login_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::AdminLoginBody>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::debug!("Admin login attempt for username: {}", body.username);

    // Verify username/password
    let valid = state.admin_user_repo
        .verify_password(&body.username, &body.password)
        .await
        .map_err(|e| {
            tracing::error!("verify_password error: {}", e);
            ApiError::Unauthorized(format!("Authentication error: {}", e))
        })?;

    tracing::debug!("verify_password result: {}", valid);

    if !valid {
        return Err(ApiError::Unauthorized("Invalid credentials".to_string()));
    }

    // Get user info for response
    let user = state.admin_user_repo
        .find_by_username(&body.username)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Failed to get user: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("User not found".to_string()))?;

    // Generate token with admin role
    let token = crate::api::generate_token(&body.username, vec!["admin".to_string()], vec![])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    tracing::info!("Admin login success for username: {}", body.username);

    let response = crate::api::models::AdminLoginResponse {
        token,
        token_type: "Bearer".to_string(),
        expires_in: 86400,
        user: crate::api::models::AdminUserInfo {
            id: user.id.to_string(),
            username: user.username,
            display_name: user.display_name,
        },
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn list_audit_logs_handler(
    State(state): State<ApiState>,
    AgentContext { roles, .. }: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if !roles.iter().any(|r| r == "admin") {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state.audit_repo
        .list_with_filters(
            query.agent_id.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
            limit,
            offset,
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state.audit_repo
        .count_with_filters(
            query.agent_id.as_deref(),
            query.action.as_deref(),
            query.resource_type.as_deref(),
        )
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs.into_iter().map(|log| {
        crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            timestamp: log.timestamp.to_rfc3339(),
        }
    }).collect();

    Ok((StatusCode::OK, Json(crate::api::models::AuditLogListResponse {
        data,
        total,
        limit,
        offset,
    })))
}

pub async fn list_my_audit_logs_handler(
    State(state): State<ApiState>,
    AgentContext { agent_id, .. }: AgentContext,
    Query(query): Query<crate::api::models::AuditLogQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.offset.unwrap_or(0);

    let logs = state.audit_repo
        .list_with_filters(Some(&agent_id), query.action.as_deref(), query.resource_type.as_deref(), limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let total = state.audit_repo
        .count_with_filters(Some(&agent_id), query.action.as_deref(), query.resource_type.as_deref())
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<_> = logs.into_iter().map(|log| {
        crate::api::models::AuditLogResponse {
            id: log.id.to_string(),
            agent_id: log.agent_id,
            action: log.action,
            resource_type: log.resource_type,
            resource_id: log.resource_id,
            details: log.details,
            timestamp: log.timestamp.to_rfc3339(),
        }
    }).collect();

    Ok((StatusCode::OK, Json(crate::api::models::AuditLogListResponse {
        data,
        total,
        limit,
        offset,
    })))
}

pub async fn approve_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { roles, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    if !roles.iter().any(|r| r == "admin") {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    skill_repo.update_status(&skill_id, "published")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve skill: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: None,
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "approved"}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill approved successfully".to_string(),
        skill_id,
    })))
}

pub async fn reject_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { roles, .. }: AgentContext,
    Json(body): Json<crate::api::models::RejectSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    if !roles.iter().any(|r| r == "admin") {
        return Err(ApiError::Unauthorized("Admin access required".to_string()));
    }

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    skill_repo.update_status(&skill_id, "rejected")
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject skill: {}", e)))?;

    state.audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: None,
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "rejected", "reason": body.reason}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(crate::api::models::SkillReviewResponse {
        message: "Skill rejected".to_string(),
        skill_id,
    })))
}

// v0.4 multi-tenant handlers

use uuid::Uuid;

/// Organization handlers

pub async fn create_org_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::CreateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state.organization.create_org(body.name)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(org).unwrap())))
}

pub async fn get_org_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state.organization.get_org(org_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn list_orgs_handler(
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListOrgsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let orgs = state.organization.list_orgs(limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": orgs }))))
}

pub async fn update_org_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    Json(body): Json<crate::api::models::UpdateOrgBody>,
) -> Result<impl IntoResponse, ApiError> {
    let org = state.organization.update_org(org_id, body.name)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(org).unwrap())))
}

pub async fn delete_org_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.organization.delete_org(org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::NO_CONTENT, Json(serde_json::json!({"deleted": org_id}))))
}

/// Session handlers

pub async fn create_session_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::CreateSessionBody>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state.session.create_session(body.agent_id, body.org_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(session).unwrap())))
}

pub async fn get_session_handler(
    State(state): State<ApiState>,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state.session.get_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    match session {
        Some(s) => Ok((StatusCode::OK, Json(serde_json::to_value(s).unwrap()))),
        None => Err(ApiError::NotFound(format!("Session {} not found", session_id))),
    }
}

pub async fn list_sessions_handler(
    State(state): State<ApiState>,
    Query(query): Query<crate::api::models::ListSessionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let status = query.status.as_deref();

    let sessions = state.session.list_sessions(limit, offset, status)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": sessions }))))
}

pub async fn end_session_handler(
    State(state): State<ApiState>,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.session.end_session(session_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"ended": session_id}))))
}

pub async fn session_declare_handler(
    State(state): State<ApiState>,
    Path(session_id): Path<Uuid>,
    Json(body): Json<crate::api::models::SessionDeclareBody>,
) -> Result<impl IntoResponse, ApiError> {
    let router = state.session.declare_capabilities(session_id, body.capabilities)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(router).unwrap())))
}

/// Org Tool handlers

pub async fn register_org_tool_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::RegisterOrgToolBody>,
) -> Result<impl IntoResponse, ApiError> {
    let tool = state.org_tool.register_tool(
        body.org_id,
        body.tool_id,
        body.name,
        body.description,
        body.schema.unwrap_or(serde_json::json!({})),
        body.implementation.unwrap_or(serde_json::json!({})),
    )
    .await
    .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::CREATED, Json(serde_json::to_value(tool).unwrap())))
}

pub async fn list_org_tools_handler(
    State(state): State<ApiState>,
    Path(org_id): Path<Uuid>,
    Query(query): Query<crate::api::models::ListOrgToolsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let approved_only = query.approved_only.unwrap_or(false);
    let tools = if approved_only {
        state.org_tool.list_approved_tools(org_id).await?
    } else {
        state.org_tool.list_org_tools(org_id).await?
    };

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tools }))))
}

pub async fn list_all_org_tools_handler(
    State(state): State<ApiState>,
) -> Result<impl IntoResponse, ApiError> {
    let tools = state.org_tool.list_all()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "data": tools }))))
}

pub async fn approve_org_tool_handler(
    State(state): State<ApiState>,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.org_tool.approve_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"approved": tool_id}))))
}

pub async fn reject_org_tool_handler(
    State(state): State<ApiState>,
    Path(tool_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    state.org_tool.reject_tool(tool_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::json!({"rejected": tool_id}))))
}
