//! 技能工作流 handlers (审核/发布/回滚)

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse, Json};

use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use super::helpers::{check_skill_perm, check_skill_perm_db, require_marketplace_admin_only, ApiState};

pub async fn submit_review_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext {
        subject,
        identity_id,
        ..
    }: AgentContext,
    Json(body): Json<crate::api::models::SubmitSkillReviewBody>,
) -> Result<impl IntoResponse, ApiError> {
    // RBAC 权限校验：需要 SubmitReview 权限
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;
    check_skill_perm(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::SubmitReview,
    )
    .await?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    skill_repo
        .update_status(&skill_id, "pending_review", None, body.comment.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to submit skill for review: {}", e)))?;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_submitted_for_review".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"comment": body.comment}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill submitted for review".to_string(),
            skill_id,
        }),
    ))
}

pub async fn publish_skill_handler(
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

    // RBAC 权限校验：需要 Publish 权限（仅做内部发布）
    check_skill_perm_db(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Publish,
    )
    .await?;

    if skill.status != "approved" {
        return Err(ApiError::BadRequest(
            "Skill must be approved before publishing".to_string(),
        ));
    }

    // 内部发布：仅设置 status 为 published，不操作 visibility
    // visibility 由创建时决定，提交市场由 skill:publish_to_marketplace 单独控制
    skill_repo
        .update_status(&skill_id, "published", None, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to publish skill: {}", e)))?;

    // 确保 release tarball 存在（审核通过时已生成，此处兜底）
    let release_path = state.skill_git.releases_dir()
        .join(&skill.name)
        .join(format!("v{}.tar.gz", skill.version));
    if !release_path.exists() {
        let _ = state.skill_git.generate_release_tarball(&skill.name, &skill.version);
    }

    // 新版本发布后，旧版本不再对外可见
    let _ = sqlx::query(
        "UPDATE skills SET is_current = false WHERE name = $1 AND is_current = true AND id != $2"
    )
    .bind(&skill.name)
    .bind(&skill.id)
    .execute(state.agent_repo.pool())
    .await;

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_published".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"visibility": skill.visibility}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill published successfully".to_string(),
            skill_id,
        }),
    ))
}

/// POST /api/v1/skills/:skill_name/rollback — 版本回退（创建新版本，走审核流程）
/// 权限：skill 的作者或组织成员（与上传新版本一致）
/// 流程：Git checkout 目标 tag → commit（不打 tag）→ 创建 pending_review skill → 等待审核
pub async fn rollback_skill_handler(
    State(state): State<ApiState>,
    agent_context: AgentContext,
    axum::extract::Path((skill_name,)): axum::extract::Path<(String,)>,
    Json(body): Json<crate::api::models::RollbackSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = agent_context.require_identity()?;

    let pool = state.agent_repo.pool().clone();
    let skill_repo = crate::db::repositories::skill::SkillRepository::new(pool.clone());
    let version_repo = crate::db::repositories::version::VersionRepository::new(pool);

    // 1. 获取当前最新版本的 owner 信息，并校验权限（作者或组织成员）
    let skill_list = skill_repo
        .list_by_name(&skill_name)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to lookup skill: {}", e)))?;
    let latest = skill_list
        .first()
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_name)))?;

    // 权限校验：必须是作者或组织成员
    let is_author = latest.author_identity_id == Some(identity_id);
    let is_owner_user = latest.owner_type == "user" && latest.owner_id == Some(identity_id);
    if !is_author && !is_owner_user {
        // 检查是否为组织成员
        let org_ids = state
            .permission
            .get_user_org_ids(identity_id)
            .await
            .unwrap_or_default();
        let is_org_member = latest.owner_type == "organization"
            && latest.owner_id.map_or(false, |oid| org_ids.contains(&oid));
        if !is_org_member {
            return Err(ApiError::Forbidden(
                "Only the skill author or organization member can rollback".to_string(),
            ));
        }
    }

    // 2. 执行 Git 回退：从 tag 恢复文件 + commit only（不打 tag） + 写入 skill_versions
    let result = state
        .skill_git
        .rollback_version_commit_only(&skill_name, &body.version, identity_id, &version_repo)
        .map_err(|e| match e {
            crate::models::error::AppError::SkillNotFound(_) => ApiError::NotFound(format!(
                "Version {} not found for skill {}",
                body.version, skill_name
            )),
            other => ApiError::BadRequest(format!("Rollback failed: {}", other)),
        })?;

    // 3. 读取恢复后的 SKILL.md 获取元数据
    let repo_dir = state.skill_git.repo_path(&skill_name);
    let skill_md_content = std::fs::read_to_string(repo_dir.join("SKILL.md"))
        .map_err(|e| ApiError::InternalError(format!("Failed to read restored SKILL.md: {}", e)))?;
    let meta = crate::services::skill_git::parse_skill_md_frontmatter(&skill_md_content)
        .map_err(|e| ApiError::BadRequest(format!("Failed to parse restored SKILL.md: {}", e)))?;

    // 4. 通过 RegistryService 创建新版本的 skill 记录（状态为 pending_review）
    let new_skill = crate::models::skill::NewSkill {
        name: skill_name.clone(),
        description: meta.description,
        tags: meta.tags,
        content: skill_md_content,
        version: result.new_version.clone(),
        git_url: None,
        visibility: Some(
            skill_list
                .iter()
                .find_map(|s| match s.visibility.as_str() {
                    "private" => Some(crate::models::skill_policy::Visibility::Private),
                    "org_visible" => Some(crate::models::skill_policy::Visibility::OrgVisible),
                    "marketplace" => Some(crate::models::skill_policy::Visibility::Marketplace),
                    "shared" => Some(crate::models::skill_policy::Visibility::Shared),
                    _ => None,
                })
                .unwrap_or(crate::models::skill_policy::Visibility::OrgVisible),
        ),
        tools: None,
        owner_type: latest.owner_type.clone(),
        owner_id: latest.owner_id,
        author_identity_id: Some(identity_id),
    };
    let skill = state
        .registry
        .create_skill(new_skill, &agent_context.subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create rolled-back skill: {}", e)))?;

    // 5. 审计日志
    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(agent_context.subject),
            action: "skill_rollback".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill.id.clone()),
            details: serde_json::json!({
                "skill_name": skill_name,
                "from_version": result.from_version,
                "target_version": result.target_version,
                "new_version": result.new_version,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!(
                "Skill {} rollback from {} to {} submitted for review (new version: {})",
                skill_name, result.from_version, result.target_version, result.new_version
            ),
            "skill_name": skill_name,
            "from_version": result.from_version,
            "target_version": result.target_version,
            "new_version": result.new_version,
            "skill_id": skill.id,
        })),
    ))
}

/// POST /api/v1/admin/skills/:id/unpublish — 管理员下架已发布的 Skill
/// 新逻辑: 将 marketplace_status 设为 'delisted'，保留 internal status 不变
pub async fn admin_unpublish_skill_handler(
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

    if skill.marketplace_status.as_deref() != Some("listed") {
        return Err(ApiError::BadRequest(
            "Only listed marketplace skills can be delisted".to_string(),
        ));
    }

    // 下架: 改 marketplace_status 为 delisted，恢复原始 visibility
    let pre_visibility = skill.pre_marketplace_visibility.as_deref().unwrap_or("private");
    skill_repo
        .update_marketplace_status(&skill_id, Some("delisted"))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delist skill: {}", e)))?;

    skill_repo
        .update(&skill_id, None, None, None, Some(pre_visibility))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to restore visibility: {}", e)))?;

    // 向后兼容：同时设置 admin_unpublished 标记
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
            action: "skill_unpublished".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "previous_marketplace_status": "listed",
                "new_marketplace_status": "delisted",
                "restored_visibility": pre_visibility,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Skill {} delisted from marketplace", skill.name),
            "skill_id": skill_id,
            "marketplace_status": "delisted",
        })),
    ))
}

/// POST /api/v1/admin/skills/:id/publish — 管理员直接上架 Skill 到市场（绕过审核）
/// 新逻辑: 使用 marketplace_status 而不是 visibility
pub async fn admin_publish_skill_handler(
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

    if skill.marketplace_status.as_deref() == Some("listed") {
        return Err(ApiError::BadRequest(
            "Skill is already listed on marketplace".to_string(),
        ));
    }

    // 只在未 published 时更新状态
    if skill.status != "published" {
        skill_repo
            .update_status(&skill_id, "published", None, None)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to publish skill: {}", e)))?;

        // 新版本发布后，旧版本不再对外可见
        let _ = sqlx::query(
            "UPDATE skills SET is_current = false WHERE name = $1 AND is_current = true AND id != $2"
        )
        .bind(&skill.name)
        .bind(&skill.id)
        .execute(state.agent_repo.pool())
        .await;
    }

    // 如果该 Skill 之前未经过审核流程（没有 Git tag），现在补充版本管理
    {
        let repo_dir = state.skill_git.repo_path(&skill.name);
        let tag_name = format!("v{}", skill.version);
        let tag_exists = std::process::Command::new("git")
            .current_dir(&repo_dir)
            .args(["rev-parse", &tag_name])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !tag_exists && repo_dir.join(".git").exists() {
            let tag_msg = format!("v{}: Admin published", skill.version);
            let _ = state.skill_git.git_tag_approved(&repo_dir, &tag_name, &tag_msg);
            let _ = state.skill_git.generate_release_tarball(&skill.name, &skill.version);

            // 统计文件数和总大小（从 git repo 统计）
            let (file_count, total_size_bytes) = state.registry.count_skill_files(&repo_dir)
                .unwrap_or((0, 0));

            use crate::db::repositories::version::{VersionRepository, NewSkillVersion};
            let version_repo = VersionRepository::new(state.agent_repo.pool().clone());
            let _ = version_repo.create(NewSkillVersion {
                skill_name: skill.name.clone(),
                version: skill.version.clone(),
                git_commit_hash: None,
                git_tag: Some(tag_name),
                changelog: Some(tag_msg),
                file_count: file_count as i32,
                total_size_bytes: total_size_bytes as i64,
                uploaded_by: agent_context.require_identity().ok(),
                git_remote_url: None,
            }).await;
        }
    }

    // 淇濆瓨褰撳墠 visibility 浣滀负 pre_marketplace_visibility
    let current_visibility = skill.visibility.clone();
    skill_repo
        .set_pre_marketplace_visibility(&skill_id, Some(&current_visibility))
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to save pre-marketplace visibility: {}", e)))?;

    // 设置为 marketplace 可见性
    skill_repo
        .update(&skill_id, None, None, None, Some("marketplace"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set marketplace visibility: {}", e))
        })?;

    // 设置 marketplace_status 为 listed
    skill_repo
        .update_marketplace_status(&skill_id, Some("listed"))
        .await
        .map_err(|e| {
            ApiError::BadRequest(format!("Failed to set marketplace status: {}", e))
        })?;

    // 清除 admin_unpublished 标记
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
            action: "skill_admin_published".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({
                "skill_name": skill.name,
                "previous_marketplace_status": skill.marketplace_status,
                "new_marketplace_status": "listed",
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": format!("Skill {} listed on marketplace", skill.name),
            "skill_id": skill_id,
            "status": "published",
            "marketplace_status": "listed",
        })),
    ))
}

/// POST /api/v1/skills/:id/submit-to-marketplace - 提交已发布 Skill 到市场审核
pub async fn approve_skill_handler(
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

    // RBAC 权限校验：需要 Approve 权限（自动校验不能审核自己的 Skill）
    check_skill_perm_db(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Approve,
    )
    .await?;

    if skill.status != "pending_review" {
        return Err(ApiError::BadRequest(
            "Skill must be in pending_review status to approve".to_string(),
        ));
    }

    let reviewer_id = identity_id;

    skill_repo
        .update_status(&skill_id, "approved", reviewer_id, None)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to approve skill: {}", e)))?;

    // Git tag + version_repo 写入 + tarball 生成
    {
        let repo_dir = state.skill_git.repo_path(&skill.name);
        let tag_name = format!("v{}", skill.version);
        let tag_msg = format!("v{}: Approved version", skill.version);

        state.skill_git.git_tag_approved(&repo_dir, &tag_name, &tag_msg)
            .map_err(|e| ApiError::InternalError(format!("Failed to tag version: {}", e)))?;

        // 统计文件数和总大小（从 git repo 统计）
        let (file_count, total_size_bytes) = state.registry.count_skill_files(&repo_dir)
            .unwrap_or((0, 0));

        use crate::db::repositories::version::{VersionRepository, NewSkillVersion};
        let version_repo = VersionRepository::new(state.agent_repo.pool().clone());
        version_repo.create(NewSkillVersion {
            skill_name: skill.name.clone(),
            version: skill.version.clone(),
            git_commit_hash: None,
            git_tag: Some(tag_name),
            changelog: Some(tag_msg),
            file_count: file_count as i32,
            total_size_bytes: total_size_bytes as i64,
            uploaded_by: reviewer_id,
            git_remote_url: None,
        }).await
        .map_err(|e| ApiError::InternalError(format!("Failed to record version: {}", e)))?;

        // 生成 tarball
        let _ = state.skill_git.generate_release_tarball(&skill.name, &skill.version);
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "approved"}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill approved successfully".to_string(),
            skill_id,
        }),
    ))
}

pub async fn reject_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext {
        subject,
        identity_id,
        ..
    }: AgentContext,
    Json(body): Json<crate::api::models::RejectSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skill = skill_repo
        .find_by_id(&skill_id)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // RBAC 权限校验：需要 Reject 权限（自动校验不能审核自己的 Skill）
    check_skill_perm_db(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Reject,
    )
    .await?;

    if skill.status != "pending_review" {
        return Err(ApiError::BadRequest(
            "Skill must be in pending_review status to reject".to_string(),
        ));
    }

    let reviewer_id = identity_id;

    skill_repo
        .update_status(&skill_id, "rejected", reviewer_id, body.reason.as_deref())
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to reject skill: {}", e)))?;

    // Git reset --soft HEAD~1 撤销审核中的 commit
    {
        let repo_dir = state.skill_git.repo_path(&skill.name);
        if repo_dir.join(".git").exists() {
            let _ = state.skill_git.git_reset_soft_head(&repo_dir);
        }
    }

    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_reviewed".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(skill_id.clone()),
            details: serde_json::json!({"action": "rejected", "reason": body.reason}),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::SkillReviewResponse {
            message: "Skill rejected".to_string(),
            skill_id,
        }),
    ))
}






