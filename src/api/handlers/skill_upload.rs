//! 技能上传与版本管理 handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use super::helpers::ApiState;
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;

pub async fn upload_skill_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut zip_data: Option<Vec<u8>> = None;
    let mut owner_type = "user".to_string();
    let mut owner_id: Option<uuid::Uuid> = None;
    let mut author_identity_id: Option<uuid::Uuid> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
                zip_data = Some(data.to_vec());
            }
            "owner_type" => {
                owner_type = field.text().await.unwrap_or_else(|_| "user".to_string());
            }
            "owner_id" => {
                let val = field.text().await.unwrap_or_default();
                if !val.is_empty() {
                    owner_id = uuid::Uuid::parse_str(&val).ok();
                }
            }
            "author_identity_id" => {
                let val = field.text().await.unwrap_or_default();
                if !val.is_empty() {
                    author_identity_id = uuid::Uuid::parse_str(&val).ok();
                }
            }
            _ => {}
        }
    }

    let zip_data = zip_data
        .ok_or_else(|| ApiError::BadRequest("ZIP file is required in 'file' field".to_string()))?;

    let upload_result = state
        .skill_git
        .process_upload(
            &zip_data,
            &subject,
            author_identity_id,
            &owner_type,
            owner_id,
            &state.registry,
            &state.search,
            &state.skill_repo,
            &state.version_repo,
        )
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    // Audit log
    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject),
            action: "skill_uploaded".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(upload_result.skill_id.clone()),
            details: serde_json::json!({
                "skill_name": upload_result.skill_name,
                "version": upload_result.version,
                "git_commit": upload_result.git_commit,
                "git_tag": upload_result.git_tag,
                "is_new_skill": upload_result.is_new_skill,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response = crate::api::models::SkillUploadResponse {
        skill_id: upload_result.skill_id,
        skill_name: upload_result.skill_name,
        version: upload_result.version,
        git_commit: upload_result.git_commit,
        git_tag: upload_result.git_tag,
        git_repo_name: upload_result.git_repo_name,
        is_new_skill: upload_result.is_new_skill,
        files: upload_result.files,
        message: "Skill uploaded successfully".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

// --- Skill Upload Preview & Confirm Handlers ---

/// POST /api/v1/skills/upload/preview — 上传 ZIP 仅解压预览，不提交
pub async fn upload_skill_preview_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let mut zip_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let data = field
                .bytes()
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to read file: {}", e)))?;
            zip_data = Some(data.to_vec());
        }
    }

    let zip_data = zip_data
        .ok_or_else(|| ApiError::BadRequest("ZIP file is required in 'file' field".to_string()))?;

    let preview = state
        .skill_git
        .preview_upload(&zip_data)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    let response = crate::api::models::SkillUploadPreviewResponse {
        preview_id: preview.preview_id,
        metadata: crate::api::models::PreviewMetadataResponse {
            name: preview.metadata.name,
            description: preview.metadata.description,
            version: preview.metadata.version.unwrap_or_default(),
            tags: preview.metadata.tags,
            dependencies: preview.metadata.dependencies,
            compatibility: preview.metadata.compatibility,
        },
        files: preview
            .files
            .into_iter()
            .map(|f| crate::api::models::PreviewFileResponse {
                path: f.path,
                size: f.size,
            })
            .collect(),
        total_files: preview.total_files,
        total_size: preview.total_size,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// GET /api/v1/skills/upload/preview/:preview_id/files/*path —  获取预览中文件内容
pub async fn get_preview_file_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    axum::extract::Path((preview_id,)): axum::extract::Path<(String,)>,
    req: axum::http::Request<axum::body::Body>,
) -> Result<impl IntoResponse, ApiError> {
    // Parse file_path from the URL path after /files/
    let uri_path = req.uri().path().to_string();
    let file_marker = "/files/";
    let file_path = match uri_path.find(file_marker) {
        Some(pos) => {
            let raw = &uri_path[pos + file_marker.len()..];
            percent_encoding::percent_decode_str(raw)
                .decode_utf8()
                .map_err(|e| ApiError::BadRequest(format!("Invalid file path encoding: {}", e)))?
                .to_string()
        }
        None => {
            return Err(ApiError::BadRequest(
                "File path not found in URL".to_string(),
            ));
        }
    };

    if file_path.is_empty() {
        return Err(ApiError::BadRequest("File path is required".to_string()));
    }

    let (content, content_type, size) = state
        .skill_git
        .get_preview_file(&preview_id, &file_path)
        .map_err(|e| match e {
        crate::models::error::AppError::FileNotFound(msg) => ApiError::NotFound(msg),
        _ => ApiError::BadRequest(e.to_string()),
    })?;

    let is_binary = content_type == "application/octet-stream";
    let text_content = if is_binary {
        format!("[Binary file: {} bytes, not displayable as text]", size)
    } else {
        String::from_utf8(content)
            .unwrap_or_else(|_| format!("[Cannot decode file as UTF-8: {} bytes]", size))
    };

    let response = crate::api::models::PreviewFileContentResponse {
        path: file_path,
        content: text_content,
        size,
        is_binary,
        content_type,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// POST /api/v1/skills/upload/preview/:preview_id/confirm - 确认上传，提交 Git + DB
pub async fn confirm_skill_upload_handler(
    State(state): State<ApiState>,
    AgentContext {
        subject,
        identity_id,
        org_id: agent_org_id,
        roles,
        ..
    }: AgentContext,
    axum::extract::Path((preview_id,)): axum::extract::Path<(String,)>,
    Json(body): Json<crate::api::models::ConfirmUploadBody>,
) -> Result<impl IntoResponse, ApiError> {
    let _identity_id =
        identity_id.ok_or_else(|| ApiError::Unauthorized("identity_id required".to_string()))?;

    // 检查是否为管理员角色（包括 super_admin, tenant_admin, org_admin 等）
    let is_admin = roles.iter().any(|r| {
        r == "admin"
            || r == "tenant_admin"
            || r == "org_admin"
            || r == "super_admin"
            || r.ends_with("_admin")
    });

    // 推断 owner_type：body 显式 → 自动（有 agent_org_id → organization，否则 user）
    let effective_owner_type = body.owner_type.as_deref().unwrap_or_else(|| {
        if agent_org_id.is_some() {
            "organization"
        } else {
            "user"
        }
    });

    let (owner_type, owner_id) = if effective_owner_type == "organization" {
        let org_id = body
            .organization_id
            .or(body.owner_id)
            .or(agent_org_id)
            .ok_or_else(|| {
                ApiError::BadRequest(
                    "organization_id is required when owner_type is organization".to_string(),
                )
            })?;

        // 验证用户属于该组织（admin 跳过组织成员校验）
        if !is_admin {
            let is_member = state
                .permission
                .is_org_member(_identity_id, org_id)
                .await
                .map_err(|e| ApiError::InternalError(e.to_string()))?;
            if !is_member {
                return Err(ApiError::Forbidden(
                    "You must be a member of this organization to create a Skill".to_string(),
                ));
            }
        }

        ("organization".to_string(), Some(org_id))
    } else {
        // 个人用户创建 Skill 时，自动设置为本人所有
        ("user".to_string(), Some(_identity_id))
    };

    let author_identity_id = body.author_identity_id.or(Some(_identity_id));

    let upload_result = state
        .skill_git
        .confirm_upload_from_preview(
            &preview_id,
            &subject,
            author_identity_id,
            &owner_type,
            owner_id,
            &state.registry,
            &state.search,
            &state.skill_repo,
            &state.version_repo,
        )
        .await
        .map_err(|e| match e {
            crate::models::error::AppError::ValidationError(ref msg)
                if msg.contains("瀹屽叏鐩稿悓") =>
            {
                ApiError::BadRequest(msg.clone())
            }
            other => ApiError::BadRequest(other.to_string()),
        })?;

    // Audit log
    state
        .audit_repo
        .create(crate::db::repositories::audit::NewAuditLog {
            agent_id: Some(subject.clone()),
            action: "skill_uploaded".to_string(),
            resource_type: "skill".to_string(),
            resource_id: Some(upload_result.skill_id.clone()),
            details: serde_json::json!({
                "skill_name": upload_result.skill_name,
                "version": upload_result.version,
                "git_commit": upload_result.git_commit,
                "git_tag": upload_result.git_tag,
                "is_new_skill": upload_result.is_new_skill,
            }),
        })
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let response = crate::api::models::SkillUploadResponse {
        skill_id: upload_result.skill_id,
        skill_name: upload_result.skill_name,
        version: upload_result.version,
        git_commit: upload_result.git_commit,
        git_tag: upload_result.git_tag,
        git_repo_name: upload_result.git_repo_name,
        is_new_skill: upload_result.is_new_skill,
        files: upload_result.files,
        message: "Skill uploaded successfully".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/skills/:name/versions - list versions for a skill by name
pub async fn list_skill_versions_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
    Query(query): Query<crate::api::models::ListVersionsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.offset.unwrap_or(0);

    let versions = state
        .version_repo
        .list_by_name(&skill_name, limit, offset)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let data: Vec<crate::api::models::SkillVersionResponse> = versions
        .into_iter()
        .map(|v| crate::api::models::SkillVersionResponse {
            id: v.id.to_string(),
            skill_name: v.skill_name,
            version: v.version,
            git_commit_hash: v.git_commit_hash,
            git_tag: v.git_tag,
            changelog: v.changelog,
            file_count: v.file_count,
            total_size_bytes: v.total_size_bytes,
            uploaded_by: v.uploaded_by,
            git_remote_url: v.git_remote_url,
            created_at: v.created_at.to_rfc3339(),
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

/// GET /api/v1/skills/:name/versions/diff - diff between two versions
pub async fn get_skill_version_diff_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(skill_name): Path<String>,
    Query(query): Query<crate::api::models::VersionDiffQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let diff = state
        .skill_git
        .get_version_diff(&skill_name, &query.from, &query.to)
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "skill_name": skill_name,
            "from_version": query.from,
            "to_version": query.to,
            "diff": diff,
        })),
    ))
}
