//! 用户认证与个人信息 handlers

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use super::helpers::{build_tenant_role_infos, ApiState};
use crate::api::error::ApiError;
use crate::api::jwt::AgentContext;
use crate::models::tenant::NewTenant;
use crate::utils::slugify;
use crate::TenantMode;

pub async fn user_login_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::UserLoginBody>,
) -> Result<impl IntoResponse, ApiError> {
    let rate_key = format!("user_login:{}", body.username);
    if !state.login_rate_limiter.check(&rate_key).await {
        return Err(ApiError::TooManyRequests(
            "Too many login attempts. Please try again later.".to_string(),
        ));
    }

    // 合并 verify + get 为一次 DB 查询
    let user = state
        .identity
        .verify_password_and_get_user(&body.username, &body.password)
        .await
        .map_err(|e| ApiError::Unauthorized(format!("Authentication error: {}", e)))?
        .ok_or_else(|| ApiError::Unauthorized("Invalid credentials".to_string()))?;

    // 检查账号状态
    if user.status != crate::models::identity::IdentityStatus::Active {
        return Err(ApiError::Unauthorized(format!(
            "Account is {}. Please contact administrator.",
            user.status
        )));
    }

    // 获取用户所在组织
    let orgs = state
        .permission
        .get_user_orgs(user.id)
        .await
        .unwrap_or_default();
    let organizations: Vec<crate::api::models::UserOrgInfo> = orgs
        .into_iter()
        .map(|o| crate::api::models::UserOrgInfo {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: o.role,
        })
        .collect();

    // 构建权限上下文，获取 system_roles 和 tenant_roles（必须在 JWT 生成之前，以判断 is_admin）
    let perm_ctx = state.permission.build_context(user.id).await.unwrap_or(
        crate::services::permission::PermissionContext {
            identity_id: user.id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        },
    );

    let system_roles: Vec<String> = perm_ctx.system_roles.into_iter().collect();

    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    // is_admin: 同时检查 is_system_admin 列、system_role_assignments 和 tenant_admin
    let is_admin = user.is_system_admin
        || system_roles
            .iter()
            .any(|r| r == "super_admin" || r == "marketplace_admin")
        || tenant_roles.iter().any(|r| r.role_name == "tenant_admin");

    // JWT roles: admin 用户需包含 "admin" 角色以启用 require_admin 快速路径
    let jwt_roles: Vec<&str> = if is_admin {
        vec!["user", "admin"]
    } else {
        vec!["user"]
    };
    let token = crate::api::jwt::generate_identity_token(user.id, &jwt_roles, &[])
        .map_err(|e| ApiError::Unauthorized(format!("{:?}", e)))?;

    tracing::info!("Login success for username: {}", body.username,);

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserLoginResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: 86400,
            user: crate::api::models::UserInfoResponse {
                id: user.id,
                username: user.username.unwrap_or_else(|| user.name.clone()),
                display_name: user.display_name,
                email: user.email,
                avatar_url: user.avatar_url,
                identity_type: user.identity_type.to_string(),
                is_admin,
                organizations,
                system_roles,
                tenant_roles,
                created_at: user.created_at,
            },
        }),
    ))
}

pub async fn user_register_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::UserRegisterBody>,
) -> Result<impl IntoResponse, ApiError> {
    let existing = state
        .identity
        .get_by_username(&body.username)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;

    if existing.is_some() {
        return Err(ApiError::BadRequest(format!(
            "Username '{}' already exists",
            body.username
        )));
    }

    // SaaS 模式验证：必须有租户名称
    let is_saas = state.tenant_config.mode == TenantMode::Sas;
    if is_saas {
        let tenant_name = body.tenant_name.as_ref().ok_or_else(|| {
            ApiError::BadRequest("Tenant name is required in SaaS mode".to_string())
        })?;

        // 验证租户名称长度：最小 2 字符，最大 50 字符
        if tenant_name.len() < 2 {
            return Err(ApiError::BadRequest(
                "Tenant name must be at least 2 characters".to_string(),
            ));
        }
        if tenant_name.len() > 50 {
            return Err(ApiError::BadRequest(
                "Tenant name must not exceed 50 characters".to_string(),
            ));
        }
    }

    let password_hash = bcrypt::hash(&body.password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?;

    let new_identity = crate::models::identity::NewIdentity {
        identity_type: crate::models::identity::IdentityType::User,
        name: body.username.clone(),
        external_id: None,
        username: Some(body.username.clone()),
        display_name: body.display_name.clone().or(Some(body.username.clone())),
        email: body.email,
        avatar_url: None,
        password_hash: Some(password_hash),
        is_system_admin: false,
        metadata: None,
    };

    let user = state
        .identity
        .create(new_identity)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to create user: {}", e)))?;

    // 为新注册用户赋予默认 skill_user 角色
    let _ = state
        .system_role_assignment
        .assign(user.id, "skill_user", None)
        .await;

    // SaaS 模式：创建租户并分配 tenant_admin 角色
    let mut tenant_roles: Vec<crate::api::models::TenantRoleInfo> = vec![];
    if is_saas {
        if let Some(tenant_name) = &body.tenant_name {
            // 生成租户 slug
            let base_slug = slugify(tenant_name);
            let slug = format!("{}-{}", base_slug, &user.id.to_string()[..8]);

            let new_tenant = NewTenant {
                name: tenant_name.clone(),
                slug: slug.clone(),
                billing_plan: Some("free".to_string()),
                sso_config: None,
                settings: serde_json::json!({}),
            };

            let tenant = state
                .tenant
                .create(new_tenant)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to create tenant: {}", e)))?;

            // 分配 tenant_admin 角色
            state
                .tenant_role_assignment
                .assign(user.id, tenant.id, "tenant_admin", Some(user.id))
                .await
                .map_err(|e| {
                    ApiError::BadRequest(format!("Failed to assign tenant admin: {}", e))
                })?;

            tracing::info!(
                "Created tenant '{}' (id={}) for user '{}' (id={})",
                tenant.name,
                tenant.id,
                user.username.as_ref().unwrap_or(&user.name),
                user.id
            );

            tenant_roles.push(crate::api::models::TenantRoleInfo {
                tenant_id: tenant.id,
                tenant_name: tenant.name,
                role_name: "tenant_admin".to_string(),
            });
        }
    }

    let token = crate::api::jwt::generate_identity_token(user.id, &["user"], &[])
        .map_err(|e| ApiError::InternalError(format!("{:?}", e)))?;

    Ok((
        StatusCode::CREATED,
        Json(crate::api::models::UserLoginResponse {
            token,
            token_type: "Bearer".to_string(),
            expires_in: 86400,
            user: crate::api::models::UserInfoResponse {
                id: user.id,
                username: user.username.unwrap_or_else(|| user.name.clone()),
                display_name: user.display_name,
                email: user.email,
                avatar_url: user.avatar_url,
                identity_type: user.identity_type.to_string(),
                is_admin: true, // SaaS 模式下注册即管理员
                organizations: vec![],
                system_roles: vec![],
                tenant_roles,
                created_at: user.created_at,
            },
        }),
    ))
}

// Password reset handlers
pub async fn forgot_password_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::ForgotPasswordBody>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .identity
        .get_by_email(&body.email)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("No account found with that email".to_string()))?;

    // Generate short-lived reset token (60 minutes)
    let token =
        crate::api::jwt::generate_short_lived_token(&user.id.to_string(), "password_reset", 60)?;

    // In production, send this token via email
    // For now, return it directly (self-service for MVP)
    Ok((
        StatusCode::OK,
        Json(crate::api::models::ForgotPasswordResponse {
            message: "Password reset token generated. Use this token to reset your password."
                .to_string(),
            reset_token: token,
        }),
    ))
}

pub async fn reset_password_handler(
    State(state): State<ApiState>,
    Json(body): Json<crate::api::models::ResetPasswordBody>,
) -> Result<impl IntoResponse, ApiError> {
    // Verify the reset token
    let user_id_str = crate::api::jwt::verify_purpose_token(&body.token, "password_reset")?;
    let user_id = uuid::Uuid::parse_str(&user_id_str)
        .map_err(|_| ApiError::BadRequest("Invalid token subject".to_string()))?;

    // Validate new password
    if body.new_password.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters".to_string(),
        ));
    }

    // Hash new password
    let password_hash = bcrypt::hash(&body.new_password, bcrypt::DEFAULT_COST)
        .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?;

    // Update password
    let update = crate::models::identity::IdentityUpdate {
        password_hash: Some(password_hash),
        ..Default::default()
    };
    state
        .identity
        .update(user_id, update)
        .await
        .map_err(|_| ApiError::InternalError("Failed to update password".to_string()))?;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::ResetPasswordResponse {
            message: "Password has been reset successfully".to_string(),
        }),
    ))
}

// Account deletion
pub async fn delete_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    // Soft delete: set status to "deleted"
    let update = crate::models::identity::IdentityUpdate {
        status: Some(crate::models::identity::IdentityStatus::Deleted),
        ..Default::default()
    };
    state
        .identity
        .update(id, update)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to delete account: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"deleted": true, "message": "Account deleted successfully"})),
    ))
}

pub async fn get_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let user = state
        .identity
        .get(id)
        .await
        .map_err(|e| ApiError::NotFound(format!("User not found: {}", e)))?
        .ok_or_else(|| ApiError::NotFound("User not found".to_string()))?;

    let orgs = state
        .permission
        .get_user_orgs(user.id)
        .await
        .unwrap_or_default();
    let organizations: Vec<crate::api::models::UserOrgInfo> = orgs
        .into_iter()
        .map(|o| crate::api::models::UserOrgInfo {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: o.role,
        })
        .collect();

    // 构建权限上下文
    let perm_ctx = state.permission.build_context(user.id).await.unwrap_or(
        crate::services::permission::PermissionContext {
            identity_id: user.id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        },
    );
    let system_roles: Vec<String> = perm_ctx.system_roles.into_iter().collect();
    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: user.email,
            avatar_url: user.avatar_url,
            identity_type: if user.is_system_admin
                || system_roles
                    .iter()
                    .any(|r| r == "super_admin" || r == "marketplace_admin")
                || tenant_roles.iter().any(|r| r.role_name == "tenant_admin")
            {
                "admin".to_string()
            } else {
                user.identity_type.to_string()
            },
            is_admin: user.is_system_admin
                || system_roles
                    .iter()
                    .any(|r| r == "super_admin" || r == "marketplace_admin")
                || tenant_roles.iter().any(|r| r.role_name == "tenant_admin"),
            organizations,
            system_roles,
            tenant_roles,
            created_at: user.created_at,
        }),
    ))
}

/// GET /users/me/permissions - 权限刷新端点
/// 返回当前用户在各级别的角色及所有可用权限码
pub async fn get_my_permissions_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let perm_ctx = state
        .permission
        .build_context(id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    let system_roles: Vec<String> = perm_ctx.system_roles.clone().into_iter().collect();

    // tenant roles with names (batch query)
    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    // org roles (from PermissionContext which already has names from org_membership)
    let org_roles: Vec<crate::api::models::OrgRoleInfo> = perm_ctx
        .org_roles
        .iter()
        .map(|(org_id, role_name)| crate::api::models::OrgRoleInfo {
            org_id: *org_id,
            org_name: String::new(), // org name not in PermissionContext; will be empty here
            role_name: role_name.clone(),
        })
        .collect();

    // group roles
    let group_roles: Vec<crate::api::models::GroupRoleInfo> = perm_ctx
        .group_roles
        .iter()
        .map(|(group_id, role_name)| crate::api::models::GroupRoleInfo {
            group_id: *group_id,
            group_name: String::new(), // group name not in PermissionContext
            role_name: role_name.clone(),
        })
        .collect();

    let permissions = state
        .permission
        .collect_all_permissions(&perm_ctx)
        .await
        .unwrap_or_default();

    Ok((
        StatusCode::OK,
        Json(crate::api::models::MyPermissionsResponse {
            system_roles,
            tenant_roles,
            org_roles,
            group_roles,
            permissions,
        }),
    ))
}

pub async fn update_user_me_handler(
    State(state): State<ApiState>,
    AgentContext { subject, .. }: AgentContext,
    Json(body): Json<crate::api::models::UpdateUserBody>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id = uuid::Uuid::parse_str(&subject)
        .map_err(|_| ApiError::BadRequest("Invalid subject in token".to_string()))?;

    let password_hash = match body.password {
        Some(pw) => Some(
            bcrypt::hash(&pw, bcrypt::DEFAULT_COST)
                .map_err(|e| ApiError::BadRequest(format!("Failed to hash password: {}", e)))?,
        ),
        None => None,
    };

    let update = crate::models::identity::IdentityUpdate {
        display_name: body.display_name,
        email: body.email,
        avatar_url: body.avatar_url,
        password_hash,
        name: None,
        status: None,
        is_system_admin: None,
        metadata: None,
    };

    let user = state
        .identity
        .update(identity_id, update)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to update user: {}", e)))?;

    let orgs = state
        .permission
        .get_user_orgs(user.id)
        .await
        .unwrap_or_default();
    let organizations: Vec<crate::api::models::UserOrgInfo> = orgs
        .into_iter()
        .map(|o| crate::api::models::UserOrgInfo {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: o.role,
        })
        .collect();

    // 构建权限上下文
    let perm_ctx = state.permission.build_context(user.id).await.unwrap_or(
        crate::services::permission::PermissionContext {
            identity_id: user.id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        },
    );
    let system_roles: Vec<String> = perm_ctx.system_roles.into_iter().collect();
    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: user.email,
            avatar_url: user.avatar_url,
            identity_type: user.identity_type.to_string(),
            is_admin: user.is_system_admin
                || system_roles
                    .iter()
                    .any(|r| r == "super_admin" || r == "marketplace_admin")
                || tenant_roles.iter().any(|r| r.role_name == "tenant_admin"),
            organizations,
            system_roles,
            tenant_roles,
            created_at: user.created_at,
        }),
    ))
}

pub async fn get_user_orgs_handler(
    State(state): State<ApiState>,
    AgentContext { identity_id, .. }: AgentContext,
) -> Result<impl IntoResponse, ApiError> {
    let uuid_id =
        identity_id.ok_or_else(|| ApiError::Unauthorized("Identity required".to_string()))?;

    // Build permission context for RBAC-based org visibility
    let perm_ctx = state
        .permission
        .build_context(uuid_id)
        .await
        .unwrap_or_else(|_| crate::services::permission::PermissionContext {
            identity_id: uuid_id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        });

    let tenant_admin_ids: Vec<uuid::Uuid> = perm_ctx
        .tenant_roles
        .iter()
        .filter(|(_, role)| role == "tenant_admin")
        .map(|(tid, _)| *tid)
        .collect();

    let mut result_set: std::collections::HashMap<uuid::Uuid, crate::api::models::UserOrgResponse> =
        std::collections::HashMap::new();

    // Add personal org memberships（所有用户，包括 super_admin，只看自己加入的组织）
    {
        let user_orgs = state
            .permission
            .get_user_orgs(uuid_id)
            .await
            .map_err(|e| ApiError::BadRequest(format!("Failed to list user orgs: {}", e)))?;
        for o in user_orgs {
            result_set
                .entry(o.id)
                .or_insert(crate::api::models::UserOrgResponse {
                    id: o.id,
                    name: o.name,
                    slug: o.slug,
                    role: o.role,
                });
        }

        // If tenant_admin, also add all orgs under the user's tenants
        for tenant_id in tenant_admin_ids {
            let tenant_orgs = state
                .organization
                .list_orgs_by_tenant(tenant_id, 1000, 0)
                .await
                .map_err(|e| ApiError::BadRequest(format!("Failed to list tenant orgs: {}", e)))?;
            for o in tenant_orgs {
                result_set
                    .entry(o.id)
                    .or_insert(crate::api::models::UserOrgResponse {
                        id: o.id,
                        name: o.name,
                        slug: o.slug,
                        role: "admin".to_string(),
                    });
            }
        }
    }

    let mut result: Vec<crate::api::models::UserOrgResponse> = result_set.into_values().collect();
    // Sort by name for stable UI
    result.sort_by(|a, b| a.name.cmp(&b.name));

    Ok((StatusCode::OK, Json(result)))
}

pub async fn list_my_skills_handler(
    State(state): State<ApiState>,
    AgentContext { identity_id, .. }: AgentContext,
    Query(_query): Query<crate::api::models::ListSkillsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let identity_id =
        identity_id.ok_or_else(|| ApiError::Unauthorized("Identity required".to_string()))?;

    use crate::db::repositories::skill::SkillRepository;
    let pool = state.agent_repo.pool().clone();
    let skill_repo = SkillRepository::new(pool);

    let skills = skill_repo
        .list_by_owner(identity_id)
        .await
        .map_err(|e| ApiError::BadRequest(format!("Failed to list my skills: {}", e)))?;

    Ok((StatusCode::OK, Json(skills)))
}

pub async fn get_user_by_username_handler(
    State(state): State<ApiState>,
    AgentContext { subject: _, .. }: AgentContext,
    Path(username): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let user = state
        .identity
        .get_by_username(&username)
        .await
        .map_err(|e| ApiError::NotFound(format!("User not found: {}", e)))?
        .ok_or_else(|| ApiError::NotFound(format!("User '{}' not found", username)))?;

    let orgs = state
        .permission
        .get_user_orgs(user.id)
        .await
        .unwrap_or_default();
    let organizations: Vec<crate::api::models::UserOrgInfo> = orgs
        .into_iter()
        .map(|o| crate::api::models::UserOrgInfo {
            id: o.id,
            name: o.name,
            slug: o.slug,
            role: o.role,
        })
        .collect();

    // Load system_roles for accurate is_admin determination
    let perm_ctx = state.permission.build_context(user.id).await.unwrap_or(
        crate::services::permission::PermissionContext {
            identity_id: user.id,
            system_roles: std::collections::HashSet::new(),
            tenant_roles: Vec::new(),
            org_roles: Vec::new(),
            group_roles: Vec::new(),
        },
    );
    let system_roles: Vec<String> = perm_ctx.system_roles.into_iter().collect();
    let tenant_roles = build_tenant_role_infos(&state, &perm_ctx.tenant_roles).await;

    Ok((
        StatusCode::OK,
        Json(crate::api::models::UserInfoResponse {
            id: user.id,
            username: user.username.unwrap_or_else(|| user.name.clone()),
            display_name: user.display_name,
            email: None,
            avatar_url: user.avatar_url,
            identity_type: user.identity_type.to_string(),
            is_admin: user.is_system_admin
                || system_roles
                    .iter()
                    .any(|r| r == "super_admin" || r == "marketplace_admin")
                || tenant_roles.iter().any(|r| r.role_name == "tenant_admin"),
            organizations,
            system_roles,
            tenant_roles,
            created_at: user.created_at,
        }),
    ))
}
