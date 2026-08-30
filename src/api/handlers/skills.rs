//! 技能管理 handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use super::helpers::{check_skill_perm, require_marketplace_admin, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use crate::models::{NewSkill, SkillUpdate};

pub async fn list_skills_handler(
    State(state): State<ApiState>,
    AgentContext { identity_id, .. }: AgentContext,
    Query(query): Query<crate::api::models::ListSkillsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).min(100);

    let skills = state
        .registry
        .list_skills()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // RBAC: build permission context
    let is_super = if let Some(id_id) = identity_id {
        state
            .permission
            .is_super_admin(id_id)
            .await
            .unwrap_or(false)
    } else {
        false
    };

    // 获取用户所属的 groups（用于 group_visible 过滤）
    let user_group_ids: Vec<uuid::Uuid> = if let Some(_id_id) = identity_id {
        sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT group_id FROM memberships WHERE identity_id = $1",
        )
        .fetch_all(state.agent_repo.pool())
        .await
        .unwrap_or_default()
    } else {
        vec![]
    };

    // Apply scope filters (Phase 6: role-based views)
    let mut filtered: Vec<_> = if let Some(org_id) = query.org_id {
        // Org member view: only skills owned by this org
        skills
            .into_iter()
            .filter(|s| s.owner_type == "organization" && s.owner_id == Some(org_id))
            .collect()
    } else if let Some(ref mkt_status) = query.marketplace_status {
        // Marketplace admin view: filter by marketplace_status
        skills
            .into_iter()
            .filter(|s| {
                s.marketplace_status
                    .as_ref()
                    .map(|ms| ms == mkt_status)
                    .unwrap_or(false)
            })
            .collect()
    } else if query.scope_personal.unwrap_or(false) {
        // Personal scope view: 浠呬釜浜鸿嚜宸辩殑 Skill
        skills
            .into_iter()
            .filter(|s| {
                s.owner_type == "user"
                    && identity_id.is_some()
                    && (s.owner_id == identity_id || s.author_identity_id == identity_id)
            })
            .collect()
    } else if is_super {
        skills
    } else {
        // 检查是否为市场管理员（需要看到待审核的市场 Skill）
        let is_market_admin = if let Some(id_id) = identity_id {
            state
                .permission
                .has_any_system_role(
                    id_id,
                    &["super_admin", "marketplace_admin", "marketplace_reviewer"],
                )
                .await
                .unwrap_or(false)
        } else {
            false
        };

        // 获取用户的 org_id（用于 org_visible 和 tenant_visible 过滤）
        let user_org_id = if let Some(id_id) = identity_id {
            state.permission.get_user_org_id(id_id).await.ok().flatten()
        } else {
            None
        };

        // 获取用户的租户 ID（用于 tenant_visible 过滤）
        let user_tenant_id = if let Some(id_id) = identity_id {
            state.permission.get_user_tenant_id(id_id).await.ok().flatten()
        } else {
            None
        };

        // 检查用户是否是租户管理员
        let is_tenant_admin = if let Some(id_id) = identity_id {
            state
                .permission
                .has_any_tenant_role(id_id, &["tenant_admin", "tenant_reviewer"])
                .await
                .unwrap_or(false)
        } else {
            false
        };

        // 获取用户所属的所有组织 ID（用于快速判断）
        let user_org_ids = if let Some(id_id) = identity_id {
            state.permission.get_user_org_ids(id_id).await.unwrap_or_default()
        } else {
            vec![]
        };

        skills
            .into_iter()
            .filter(|s| {
                // 作者能看到自己任何状态的技能（包括 approved 待发布的）
                let is_author = s.author_identity_id == identity_id;

                // 租户管理员可以看到组织内待发布的技能
                // 或者用户是其所属组织内的成员
                let is_org_member = s.owner_type == "organization"
                    && s.owner_id.is_some()
                    && user_org_ids.contains(&s.owner_id.unwrap());

                let is_own = is_author || is_tenant_admin || is_org_member;

                // 非作者/管理员：必须已发布
                if !is_own && s.status != "published" {
                    return false;
                }

                // Published marketplace skills visible to all
                let is_marketplace_published = matches!(
                    s.visibility,
                    crate::models::skill_policy::Visibility::Marketplace
                );

                // 根据 visibility scope 过滤
                let in_visible_scope = match &s.visibility {
                    crate::models::skill_policy::Visibility::GroupVisible => {
                        // group_visible: 需要用户在对应的 group 内
                        // 技能需要关联到 group，这需要从 skill_groups 表查询
                        // 暂时先检查用户是否属于任何 group（后续完善 group 关联）
                        !user_group_ids.is_empty()
                    }
                    crate::models::skill_policy::Visibility::OrgVisible => {
                        // org_visible: 需要用户在对应的组织内
                        user_org_id.is_some() && s.owner_id == user_org_id
                    }
                    crate::models::skill_policy::Visibility::TenantVisible => {
                        // tenant_visible: 需要用户在对应的租户内
                        user_tenant_id.is_some() && s.owner_id == user_tenant_id
                    }
                    crate::models::skill_policy::Visibility::Private => {
                        // private: 只有作者能看到
                        s.author_identity_id == identity_id
                    }
                    crate::models::skill_policy::Visibility::Marketplace
                    | crate::models::skill_policy::Visibility::Shared => true,
                };

                // 市场管理员可以看到所有已提交市场的 Skill（任何 marketplace_status）
                let is_market_admin_visible = is_market_admin && s.marketplace_status.is_some();

                is_marketplace_published || is_own || is_market_admin_visible || in_visible_scope
            })
            .collect()
    };

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
    AgentContext { identity_id, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    // RBAC 权限校验：需要 Read 权限
    check_skill_perm(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Read,
    )
    .await?;

    let stats = state.evaluator.get_stats(&skill_id).await.ok();

    let detail = crate::models::SkillDetail {
        metadata: (&skill).into(),
        content: skill.content,
        stats,
    };
    Ok((StatusCode::OK, Json(detail)))
}

pub async fn create_skill_handler(
    State(state): State<ApiState>,
    AgentContext {
        subject,
        identity_id,
        org_id: agent_org_id,
        roles,
        ..
    }: AgentContext,
    Json(body): Json<crate::api::models::CreateSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id =
        identity_id.ok_or_else(|| ApiError::Unauthorized("identity_id required".to_string()))?;

    let is_admin = roles.iter().any(|r| r == "admin");

    // owner_type: body 显式指定 > 自动推断（agent_org_id 存在 → organization，否则 user）
    let effective_owner_type = body.owner_type.as_deref().unwrap_or_else(|| {
        if agent_org_id.is_some() {
            "organization"
        } else {
            "user"
        }
    });

    let (owner_type, owner_id, default_visibility) = if effective_owner_type == "organization" {
        // organization_id: body 显式指定 > 调用者关联的组织
        let org_id = body.organization_id.or(agent_org_id).ok_or_else(|| {
            ApiError::BadRequest(
                "organization_id is required when owner_type is organization".to_string(),
            )
        })?;

        // 验证用户属于该组织（admin 跳过组织成员校验）
        if !is_admin {
            let is_member = state
                .permission
                .is_org_member(identity_id, org_id)
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            if !is_member {
                return Err(ApiError::Forbidden(
                    "浣犱笉鑳戒负涓嶅睘浜庣殑缁勭粐鍒涘缓 Skill".to_string(),
                ));
            }
        }

        (
            "organization".to_string(),
            Some(org_id),
            crate::models::skill_policy::Visibility::OrgVisible,
        )
    } else {
        // 个人用户创建 Skill 时，自动设置为本人所有
        (
            "user".to_string(),
            Some(identity_id),
            crate::models::skill_policy::Visibility::Private,
        )
    };

    // 用户显式指定的 visibility 优先，否则使用按 owner_type 的默认值
    let visibility = match body.visibility.as_deref() {
        Some("private") => crate::models::skill_policy::Visibility::Private,
        Some("group_visible") => crate::models::skill_policy::Visibility::GroupVisible,
        Some("org_visible") => crate::models::skill_policy::Visibility::OrgVisible,
        Some("tenant_visible") => crate::models::skill_policy::Visibility::TenantVisible,
        Some("marketplace") => crate::models::skill_policy::Visibility::Marketplace,
        Some("shared") => crate::models::skill_policy::Visibility::Shared,
        _ => default_visibility,
    };

    let new_skill = NewSkill {
        name: body.name,
        description: body.description,
        tags: body.tags,
        content: body.content,
        version: body.version.unwrap_or_else(|| "1.0.0".to_string()),
        git_url: body.git_url.clone(),
        visibility: Some(visibility),
        tools: body.tools.clone(),
        owner_type,
        owner_id,
        author_identity_id: Some(identity_id),
    };

    let skill = state
        .registry
        .create_skill(new_skill, &subject, &state.search)
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
    agent_context: AgentContext,
    Json(body): Json<crate::api::models::UpdateSkillBody>,
) -> Result<impl IntoResponse, ApiError> {
    let is_market_admin = require_marketplace_admin(&state, &agent_context)
        .await
        .is_ok();

    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let identity_id = agent_context.require_identity()?;
    let subject = agent_context.subject.clone();

    if !is_market_admin {
        check_skill_perm(
            &state,
            Some(identity_id),
            &skill,
            crate::services::SkillAction::Update,
        )
        .await?;
    }

    // 如果当前是市场已上架状态，编辑内容需要走审核流程
    if skill.marketplace_status.as_deref() == Some("listed")
        || skill.marketplace_status.as_deref() == Some("pending_update")
    {
        use crate::db::repositories::skill::SkillRepository;
        let pool = state.agent_repo.pool().clone();
        let skill_repo = SkillRepository::new(pool);

        // 鏋勫缓 draft_content
        let mut draft = serde_json::Map::new();
        if let Some(ref desc) = body.description {
            draft.insert(
                "description".to_string(),
                serde_json::Value::String(desc.clone()),
            );
        }
        if let Some(ref tags) = body.tags {
            draft.insert("tags".to_string(), serde_json::json!(tags));
        }
        if let Some(ref content) = body.content {
            draft.insert(
                "content".to_string(),
                serde_json::Value::String(content.clone()),
            );
        }

        skill_repo
            .save_draft_content(&skill_id, &serde_json::Value::Object(draft))
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to save draft: {}", e)))?;

        if skill.marketplace_status.as_deref() != Some("pending_update") {
            skill_repo
                .update_marketplace_status(&skill_id, Some("pending_update"))
                .await
                .map_err(|e| {
                    ApiError::BadRequest(format!("Failed to set pending_update: {}", e))
                })?;
        }

        state
            .audit_repo
            .create(crate::db::repositories::audit::NewAuditLog {
                agent_id: Some(subject),
                action: "marketplace_update_submitted".to_string(),
                resource_type: "skill".to_string(),
                resource_id: Some(skill_id.clone()),
                details: serde_json::json!({
                    "skill_name": skill.name,
                    "marketplace_status": "pending_update",
                }),
            })
            .await
            .map_err(|e| ApiError::InternalError(e.to_string()))?;

        return Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Update submitted for review",
                "skill_id": skill_id,
                "marketplace_status": "pending_update",
            })),
        ));
    }

    // 非市场 Skill：直接更新
    let visibility = body.visibility.as_ref().map(|v| match v.as_str() {
        "private" => crate::models::skill_policy::Visibility::Private,
        "group_visible" => crate::models::skill_policy::Visibility::GroupVisible,
        "org_visible" => crate::models::skill_policy::Visibility::OrgVisible,
        "tenant_visible" => crate::models::skill_policy::Visibility::TenantVisible,
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

    state
        .registry
        .update_skill(&skill_id, update, &subject, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update skill: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "message": "Skill updated successfully",
        })),
    ))
}

pub async fn delete_skill_handler(
    State(state): State<ApiState>,
    Path(skill_id): Path<String>,
    AgentContext { identity_id, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    // 权限校验：只有所有者和管理员可以删除
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;
    check_skill_perm(
        &state,
        identity_id,
        &skill,
        crate::services::SkillAction::Delete,
    )
    .await?;

    // 市场 Skill 在上架中或等待下架审核期间不允许删除
    if skill.marketplace_status.as_deref() == Some("listed")
        || skill.marketplace_status.as_deref() == Some("pending_delist")
    {
        return Err(ApiError::BadRequest(
            "Cannot delete this marketplace skill. Please delist it first.".to_string(),
        ));
    }

    state
        .registry
        .delete_skill(&skill_id, &state.search)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to delete skill: {}", e)))?;

    // 同时删除 Git 仓库目录
    let repo_dir = state.skill_git.repo_path(&skill.name);
    if repo_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&repo_dir) {
            tracing::warn!("Failed to delete git repo {}: {}", repo_dir.display(), e);
        } else {
            tracing::info!("Deleted git repo for skill {}", skill.name);
        }
    }

    // 同时删除 release tarball
    let release_path = state
        .skill_git
        .releases_dir()
        .join(&skill.name);
    if release_path.exists() {
        let _ = std::fs::remove_dir_all(&release_path);
    }

    let response = crate::api::models::MessageResponse {
        message: "Skill deleted successfully".to_string(),
    };
    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_skill_stats_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // 先确认 skill 存在
    state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Skill {} not found", skill_id)))?;

    // 获取统计；如果没有评价数据则返回默认值
    let stats = state
        .evaluator
        .get_stats(&skill_id)
        .await
        .unwrap_or_else(|_| crate::models::evaluation::SkillStats {
            skill_id: skill_id.clone(),
            success_rate: 0.0,
            avg_duration_ms: 0,
            total_evaluations: 0,
            unique_agents: 0,
            confidence: 0.0,
            tags: vec![],
            local_version: None,
            latest_version: "1.0.0".to_string(),
            upgrade_available: false,
        });

    Ok((StatusCode::OK, Json(stats)))
}

/// GET /api/v1/skills/:id/files - 列出 Skill 包中的所有文件
pub async fn list_skill_files_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let files = state
        .skill_git
        .list_files_at_version(&skill.name, &skill.version)
        .unwrap_or_default();

    Ok((StatusCode::OK, Json(serde_json::json!({ "files": files }))))
}

/// GET /api/v1/skills/:id/files/*path — 获取 Skill 包中某个文件的内容
pub async fn get_skill_file_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    axum::extract::Path((skill_id, file_path)): axum::extract::Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let skill = state
        .registry
        .get_skill(&skill_id)
        .await
        .map_err(|e| ApiError::NotFound(format!("Skill {} not found: {}", skill_id, e)))?;

    let raw = state
        .skill_git
        .get_file_at_version(&skill.name, &skill.version, &file_path)
        .map_err(|e| ApiError::NotFound(format!("File '{}' not found: {}", file_path, e)))?;

    // 检测是否为二进制文件（包含 null 字节或大量不可打印字符）
    let is_binary = raw.as_bytes().contains(&0)
        || raw
            .as_bytes()
            .iter()
            .take(1024)
            .filter(|b| !b.is_ascii_graphic() && !b.is_ascii_whitespace())
            .count()
            > 32;

    let content = if is_binary {
        let ext = std::path::Path::new(&file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let size = raw.len();
        format!("[Binary file: {} bytes, type: .{ext}]", size)
    } else {
        raw
    };

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "path": file_path,
            "content": content,
            "binary": is_binary,
        })),
    ))
}
