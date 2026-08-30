//! 评价管理 handlers

use axum::Json as JsonExt;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use super::helpers::{require_admin, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use crate::models::error::AppError;
use crate::models::evaluation::{ErrorType as EvalErrorType, EvalTag};

pub async fn create_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    JsonExt(body): JsonExt<crate::api::models::CreateEvaluationBody>,
) -> Result<
    (
        StatusCode,
        Json<crate::api::models::EvaluationCreatedResponse>,
    ),
    ApiError,
> {
    let error_type = body.error_type.as_ref().and_then(|e| match e.as_str() {
        "timeout" => Some(EvalErrorType::Timeout),
        "crash" => Some(EvalErrorType::Crash),
        "logic_error" => Some(EvalErrorType::LogicError),
        _ => Some(EvalErrorType::Other),
    });

    let tags = body
        .tags
        .iter()
        .filter_map(|t| match t.as_str() {
            "reliable" => Some(EvalTag::Reliable),
            "fast" => Some(EvalTag::Fast),
            "stable" => Some(EvalTag::Stable),
            "experimental" => Some(EvalTag::Experimental),
            _ => None,
        })
        .collect();

    let result = state
        .evaluator
        .add_evaluation(
            body.skill_id,
            subject,
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

pub async fn list_evaluations_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListEvaluationsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let skill_id = match query.skill_id.as_deref() {
        Some(id) => id,
        None => {
            return Err(ApiError::BadRequest(
                "skill_id query parameter is required".to_string(),
            ))
        }
    };

    let evals = state
        .evaluator
        .list_evaluations(skill_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::EvaluationItemResponse> = evals
        .into_iter()
        .map(|e| crate::api::models::EvaluationItemResponse {
            id: e.id,
            skill_id: e.skill_id,
            agent_id: e.agent_id,
            success: e.success,
            duration_ms: e.duration_ms,
            error_type: e.error_type.map(|et| format!("{:?}", et)),
            tags: e.tags.iter().map(|t| format!("{:?}", t)).collect(),
            timestamp: e.timestamp.to_rfc3339(),
        })
        .collect();

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "data": data,
            "total": data.len(),
        })),
    ))
}

pub async fn get_evaluation_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(eval_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let eval = state
        .evaluator
        .get_evaluation(eval_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Evaluation {} not found", eval_id)))?;

    let response = crate::api::models::EvaluationItemResponse {
        id: eval.id,
        skill_id: eval.skill_id,
        agent_id: eval.agent_id,
        success: eval.success,
        duration_ms: eval.duration_ms,
        error_type: eval.error_type.map(|et| format!("{:?}", et)),
        tags: eval.tags.iter().map(|t| format!("{:?}", t)).collect(),
        timestamp: eval.timestamp.to_rfc3339(),
    };

    Ok((StatusCode::OK, Json(response)))
}

pub async fn delete_evaluation_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    Path(eval_id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    // Check ownership: only evaluation creator or admin can delete
    let eval = state
        .evaluator
        .get_evaluation(eval_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Evaluation {} not found", eval_id)))?;

    let is_admin = require_admin(&state, &agent_context).await.is_ok();
    if !is_admin {
        let identity_id = agent_context.require_identity()?;
        let subject_str = identity_id.to_string();
        let agent_str = agent_context
            .agent_id
            .map(|id| id.to_string())
            .unwrap_or_default();
        if eval.agent_id != subject_str && eval.agent_id != agent_str {
            return Err(ApiError::Unauthorized(
                "Not authorized to delete this evaluation".to_string(),
            ));
        }
    }

    state
        .evaluator
        .delete_evaluation(eval_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "evaluation_deleted".to_string(),
            resource_type: "evaluation".to_string(),
            resource_id: Some(eval_id.to_string()),
            details: serde_json::json!({}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Evaluation deleted successfully",
            "evaluation_id": eval_id.to_string(),
        })),
    ))
}

// --- Webhook Management Handlers (Feature #11) ---
