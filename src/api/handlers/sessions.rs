//! 会话管理 handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use super::helpers::{require_admin, ApiState};
use crate::api::error::ApiError;
use crate::api::http_state::AppRouterState;
use crate::api::jwt::AgentContext;

/// Session handlers
pub async fn get_session_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state
        .session
        .get_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    match session {
        Some(s) => {
            // Check ownership: only the session owner or admin can view
            let is_admin = require_admin(&state, &agent_context).await.is_ok();
            if !is_admin {
                let identity_id = agent_context.require_identity()?;
                if s.identity_id != identity_id {
                    return Err(ApiError::Unauthorized(
                        "Not authorized to view this session".to_string(),
                    ));
                }
            }

            let enriched = enrich_session_with_meta(&state, s).await?;
            Ok((
                StatusCode::OK,
                Json(serde_json::to_value(enriched).unwrap()),
            ))
        }
        None => Err(ApiError::NotFound(format!(
            "Session {} not found",
            session_id
        ))),
    }
}

pub async fn list_sessions_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Query(query): Query<crate::api::models::ListSessionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(100);
    let offset = query.offset.unwrap_or(0);
    let status = query.status.as_deref();

    let is_admin = require_admin(&state, &agent_context).await.is_ok();
    let own_identity_id = if !is_admin {
        Some(agent_context.require_identity()?)
    } else {
        None
    };

    let sessions = state
        .session
        .list_sessions(limit, offset, status)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Non-admin users can only see their own sessions
    let filtered: Vec<_> = if let Some(identity_id) = own_identity_id {
        sessions
            .into_iter()
            .filter(|s| s.identity_id == identity_id)
            .collect()
    } else {
        sessions
    };

    // Enrich each session with identity & org names (concurrent lookups per session)
    let enriched: Vec<crate::models::session::SessionWithMeta> = futures_util::future::join_all(
        filtered
            .into_iter()
            .map(|s| enrich_session_with_meta(&state, s)),
    )
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({ "data": enriched })),
    ))
}

pub async fn end_session_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(session_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // Check ownership: only session owner or admin can end a session
    let session = state
        .session
        .get_session(session_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Session {} not found", session_id)))?;

    let is_admin = require_admin(&state, &agent_context).await.is_ok();
    if !is_admin {
        let identity_id = agent_context.require_identity()?;
        if session.identity_id != identity_id {
            return Err(ApiError::Unauthorized(
                "Not authorized to end this session".to_string(),
            ));
        }
    }

    state
        .session
        .end_session(session_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"ended": session_id})),
    ))
}

pub async fn session_declare_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(session_id): Path<Uuid>,
    Json(body): Json<crate::api::models::SessionDeclareBody>,
) -> Result<impl IntoResponse, ApiError> {
    let router = state
        .session
        .declare_capabilities(session_id, body.capabilities)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((StatusCode::OK, Json(serde_json::to_value(router).unwrap())))
}

/// Enrich a repo-level Session with identity and org names for admin display.
async fn enrich_session_with_meta(
    state: &AppRouterState,
    session: crate::db::repositories::session::Session,
) -> Result<crate::models::session::SessionWithMeta, ApiError> {
    let (identity_name, identity_display_name) = state
        .identity
        .get(session.identity_id)
        .await
        .ok()
        .flatten()
        .map(|id| (id.name.clone(), id.display_name.clone()))
        .unwrap_or_else(|| (session.identity_id.to_string(), None));

    let (org_name, tenant_name) = state
        .organization
        .get_org(session.org_id)
        .await
        .map(|org| (org.name, org.tenant_name))
        .unwrap_or_else(|_| (session.org_id.to_string(), None));

    Ok(crate::models::session::SessionWithMeta {
        id: session.id,
        identity_id: session.identity_id,
        identity_name,
        identity_display_name,
        org_id: session.org_id,
        org_name,
        tenant_name,
        status: session.status,
        tool_router: session.tool_router,
        capabilities: session.capabilities,
        created_at: session.created_at,
        last_active_at: session.last_active_at,
        ended_at: session.ended_at,
    })
}
