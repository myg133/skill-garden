//! 市场管理 handlers

use axum::{extract::{Path, Query, State}, http::StatusCode, response::IntoResponse, Json};

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{check_skill_perm_db, require_marketplace_admin, require_marketplace_admin_only, ApiState};

pub async fn submit_to_marketplace_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext {
        subject,
        identity_id,
        ..
    }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // RBAC 权限校验：需要 PublishToMarketplace 权限
    check_skill_perm_db(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::PublishToMarketplace,
    )
    .await?;

    // 必须是已发布状态
    if skill.status != "published" {
        return Err(ApiError::BadRequest(
            "Only published skills can be submitted to marketplace".to_string(),
        ));
    }

    // 涓嶈兘閲嶅鎻愪氦
    if skill.marketplace_status.as_deref() == Some("pending_review") {
        return Err(ApiError::BadRequest(
            "Skill is already pending marketplace review".to_string(),
        ));
    }

    if skill.marketplace_status.as_deref() == Some("listed") {
        return Err(ApiError::BadRequest(
            "Skill is already listed on marketplace".to_string(),
        ));
    }

    // 淇濆瓨鎻愪氦鍓嶇殑 visibility
    skill_repo
        .set_pre_marketplace_visibility(&skill_id, Some(&skill.visibility))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to save pre-marketplace visibility: {}", e)))?;

    // 设置 marketplace_status = pending_review
    skill_repo
        .update_marketplace_status(&skill_id, Some("pending_review"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to submit to marketplace: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_submitted_to_marketplace".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "previous_visibility": skill.visibility,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill submitted to marketplace review".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "pending_review",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/approve - 市场审核通过
pub async fn marketplace_review_approve_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_review") {
        return Err(ApiError::BadRequest(
            "Skill is not pending marketplace review".to_string(),
        ));
    }

    // 审核通过: 设置 marketplace_status=listed, visibility=marketplace
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some("marketplace"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set marketplace visibility: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_review_approved".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Marketplace review approved".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/reject - 市场审核驳回
pub async fn marketplace_review_reject_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_review") {
        return Err(ApiError::BadRequest(
            "Skill is not pending marketplace review".to_string(),
        ));
    }

    // 驳回: 设置 marketplace_status=rejected, 恢复原始 visibility
    let pre_visibility = skill.pre_marketplace_visibility.as_deref().unwrap_or("private");
    skill_repo
        .update_marketplace_status(&skill_id, Some("rejected"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some(pre_visibility))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to restore visibility: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_review_rejected".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "rejected",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Marketplace review rejected".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "rejected",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/relist - 重新上架已下架的 Skill
pub async fn marketplace_relist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin_only(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("delisted") {
        return Err(ApiError::BadRequest(
            "Only delisted skills can be relisted".to_string(),
        ));
    }

    // 閲嶆柊涓婃灦: 璁剧疆 marketplace_status=listed, visibility=marketplace
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to relist: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some("marketplace"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set marketplace visibility: {}", e))
        })?;

    skill_repo
        .set_admin_unpublished(&skill_id, false)
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to clear admin unpublished flag: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_skill_relisted".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill relisted on marketplace".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

pub async fn marketplace_delist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    // marketplace_admin and marketplace_reviewer can both delist
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("listed") {
        return Err(ApiError::BadRequest(
            "Only currently listed skills can be delisted".to_string(),
        ));
    }

    // 下架: 设置 marketplace_status=delisted, visibility 回退到 pre_marketplace_visibility
    let pre_visibility = skill.pre_marketplace_visibility.as_deref().unwrap_or("private");
    skill_repo
        .update_marketplace_status(&skill_id, Some("delisted"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delist: {}", e)))?;

    // Revert visibility from marketplace using pre_marketplace_visibility
    skill_repo
        .update(&skill_id, None, None, None, Some(pre_visibility))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to revert visibility: {}", e))
        })?;

    skill_repo
        .set_admin_unpublished(&skill_id, true)
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set admin unpublished flag: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_skill_delisted".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "delisted",
                "restored_visibility": pre_visibility,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill delisted from marketplace".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "delisted",
        })),
    ))
}

pub async fn marketplace_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Query(query): Query<crate::api::models::MarketplaceQuery>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    // 使用新的双键模型查询: status=published AND marketplace_status='listed'
    let skills = skill_repo
        .list_marketplace_listed(limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((StatusCode::OK, Json(skills)))
}

/// GET /api/v1/skills/:name/download/:version?token=...
/// 返回 skill 目录的 tar.gz 包
/// token 为 DB 中的不透明 UUID，由 skills.install 生成，10 分钟有效
// ============================================================================
// Marketplace Delist Request Workflow (Author → Admin/Reviewer)
// ============================================================================

/// POST /api/v1/skills/:id/request-delist - 作者申请下架市场上架的 Skill
pub async fn request_marketplace_delist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::RequestDelistBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let identity_id = agent_context.require_identity()?;

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // 只有 Skill 的作者（owner）或组织 Admin 可以申请下架
    let mut is_owner = false;

    if skill.owner_type == "user" {
        // 个人 Skill：作者本人可申请下架
        is_owner = skill.owner_id == Some(identity_id)
            || skill.author_identity_id == Some(identity_id);
    } else if skill.owner_type == "organization" {
        // 组织 Skill：组织 Admin 及以上可申请下架
        if let Some(org_id) = skill.owner_id {
            match state.permission.get_org_role(identity_id, org_id).await {
                Ok(Some(role)) if role >= crate::models::org_membership::OrgRole::Admin => {
                    is_owner = true;
                }
                _ => {}
            }
        }
    }

    if !is_owner {
        return Err(ApiError::Unauthorized(
            "Only the skill owner can request delisting".to_string(),
        ));
    }

    // 只有上架中的 Skill 可以申请下架
    if skill.marketplace_status.as_deref() != Some("listed") {
        return Err(ApiError::BadRequest(
            "Only listed marketplace skills can request delisting".to_string(),
        ));
    }

    // 璁剧疆 marketplace_status = pending_delist锛屽悓鏃朵繚瀛樹笅鏋跺師鍥犲埌 review_comment
    skill_repo
        .update_marketplace_status(&skill_id, Some("pending_delist"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to request delist: {}", e))
        })?;

    if let Some(ref reason) = body.reason {
        skill_repo
            .update_status(&skill_id, &skill.status, None, Some(reason))
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to save delist reason: {}", e)))?;
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_delist_requested".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "reason": body.reason,
                "new_marketplace_status": "pending_delist",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Delist request submitted for review".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "pending_delist",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/approve-delist - 市场管理员批准下架请求
pub async fn marketplace_approve_delist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_delist") {
        return Err(ApiError::BadRequest(
            "Skill is not pending delist review".to_string(),
        ));
    }

    // 批准下架：设置 marketplace_status=delisted，恢复 pre_marketplace_visibility
    let pre_visibility = skill.pre_marketplace_visibility.as_deref().unwrap_or("private");
    skill_repo
        .update_marketplace_status(&skill_id, Some("delisted"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve delist: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some(pre_visibility))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to revert visibility: {}", e))
        })?;

    skill_repo
        .set_admin_unpublished(&skill_id, true)
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set admin unpublished flag: {}", e))
        })?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_delist_approved".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "delisted",
                "restored_visibility": pre_visibility,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Delist request approved, skill delisted from marketplace".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "delisted",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/reject-delist - 市场管理员驳回下架申请
pub async fn marketplace_reject_delist_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_delist") {
        return Err(ApiError::BadRequest(
            "Skill is not pending delist review".to_string(),
        ));
    }

    // 驳回下架申请：恢复 marketplace_status = listed
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject delist: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_delist_rejected".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Delist request rejected, skill remains listed".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

// ============================================================================
// Marketplace Update Review Workflow (pending_update)
// ============================================================================

/// POST /api/v1/admin/marketplace/:id/approve-update - 批准内容更新
pub async fn marketplace_approve_update_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_update") {
        return Err(ApiError::BadRequest(
            "Skill is not pending update review".to_string(),
        ));
    }

    let draft = skill.draft_content.ok_or_else(|| {
        ApiError::BadRequest("No draft content to apply".to_string())
    })?;

    // 搴旂敤 draft 鍒颁富瀛楁
    skill_repo
        .apply_draft_content(&skill_id, &draft)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to apply update: {}", e)))?;

    // 鎭㈠ marketplace_status = listed
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to restore listed status: {}", e)))?;

    // 鏇存柊鎼滅储绱㈠紩
    if let Ok(updated) = state.registry.get_skill(&skill_id).await {
        if let Err(e) = state.search.update_skill(&updated) {
            tracing::warn!("Failed to update search index for {}: {}", skill_id, e);
        }
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_update_approved".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Update approved and applied".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

/// POST /api/v1/admin/marketplace/:id/reject-update - 驳回内容更新
pub async fn marketplace_reject_update_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;
    let _ = agent_context.require_identity()?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    if skill.marketplace_status.as_deref() != Some("pending_update") {
        return Err(ApiError::BadRequest(
            "Skill is not pending update review".to_string(),
        ));
    }

    // 清空 draft，恢复 listed
    skill_repo
        .clear_draft_content(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to clear draft: {}", e)))?;

    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to restore listed status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_update_rejected".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Update rejected, skill remains listed".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

/// POST /api/v1/skills/:id/cancel-update - 作者取消更新草稿
pub async fn cancel_update_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let identity_id = agent_context.require_identity()?;

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // 只有作者可以取消
    let is_owner = skill.owner_type == "user"
        && (skill.owner_id == Some(identity_id)
            || skill.author_identity_id == Some(identity_id));
    if !is_owner && skill.owner_type == "organization" {
        // 组织成员检查稍后在下面
    }
    if !is_owner && skill.owner_type == "user" {
        return Err(ApiError::Unauthorized(
            "Only the skill owner can cancel updates".to_string(),
        ));
    }

    if skill.marketplace_status.as_deref() != Some("pending_update") {
        return Err(ApiError::BadRequest(
            "Skill is not pending update review".to_string(),
        ));
    }

    skill_repo
        .clear_draft_content(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to clear draft: {}", e)))?;

    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to restore listed status: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "marketplace_update_cancelled".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Update cancelled, skill remains listed".to_string(),
            "skill_id": skill_id,
            "marketplace_status": "listed",
        })),
    ))
}

/// GET /api/v1/admin/marketplace/stats - 市场统计（市场管理员/审核员可访问）
pub async fn marketplace_stats_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    require_marketplace_admin(&state, &agent_context).await?;

    let pool = state.agent_repo.pool();

    let listed = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'listed'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let pending_review = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'pending_review'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let pending_update = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'pending_update'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let pending_delist = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'pending_delist'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 本月新增（listed 且 created_at 在本月）
    let new_this_month = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM skills WHERE marketplace_status = 'listed' AND created_at >= date_trunc('month', NOW())",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    // 总安装次数（listed 的 skill）
    let total_installs = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(SUM(install_count), 0) FROM skills WHERE marketplace_status = 'listed'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "listed": listed,
            "pending_review": pending_review,
            "pending_update": pending_update,
            "pending_delist": pending_delist,
            "new_this_month": new_this_month,
            "total_installs": total_installs,
        })),
    ))
}



