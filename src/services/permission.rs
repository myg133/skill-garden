use std::collections::HashSet;
use uuid::Uuid;

use crate::db::repositories::group::GroupRepository;
use crate::db::repositories::{
    GroupPermissionOverrideRepository, IdentityRepository, OrgMembershipRepository,
    RolePermissionRepository, SystemRoleAssignmentRepository,
};
use crate::models::error::AppError;
use crate::models::org_membership::OrgRole;

#[derive(Debug, Clone)]
pub struct PermissionContext {
    pub identity_id: Uuid,
    pub system_roles: HashSet<String>,
    pub org_roles: Vec<(Uuid, String)>,
    pub group_roles: Vec<(Uuid, String)>,
}

#[derive(Debug, Clone)]
pub struct ResourceScope {
    pub owner_type: Option<String>,
    pub owner_id: Option<Uuid>,
    pub author_identity_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub group_id: Option<Uuid>,
}

/// Skill 操作类型，用于权限校验
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SkillAction {
    Read,
    Update,
    Delete,
    SubmitReview,
    Approve,
    Reject,
    Publish,
}

#[derive(Clone)]
pub struct PermissionService {
    system_role_repo: SystemRoleAssignmentRepository,
    org_membership_repo: OrgMembershipRepository,
    role_permission_repo: RolePermissionRepository,
    group_perm_override_repo: GroupPermissionOverrideRepository,
    group_repo: GroupRepository,
    identity_repo: IdentityRepository,
}

impl std::fmt::Debug for PermissionService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PermissionService").finish()
    }
}

impl PermissionService {
    pub fn new(
        system_role_repo: SystemRoleAssignmentRepository,
        org_membership_repo: OrgMembershipRepository,
        role_permission_repo: RolePermissionRepository,
        group_perm_override_repo: GroupPermissionOverrideRepository,
        group_repo: GroupRepository,
        identity_repo: IdentityRepository,
    ) -> Self {
        Self {
            system_role_repo,
            org_membership_repo,
            role_permission_repo,
            group_perm_override_repo,
            group_repo,
            identity_repo,
        }
    }

    pub async fn is_super_admin(&self, identity_id: Uuid) -> Result<bool, AppError> {
        self.system_role_repo
            .has_system_role(identity_id, "super_admin")
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 检查身份是否为系统管理员（通过 identities.is_system_admin 字段）
    pub async fn is_system_admin(&self, identity_id: Uuid) -> Result<bool, AppError> {
        self.identity_repo
            .is_system_admin(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 检查用户是否是指定组织的成员
    pub async fn is_org_member(&self, identity_id: Uuid, org_id: Uuid) -> Result<bool, AppError> {
        self.org_membership_repo
            .is_member(identity_id, org_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 获取用户在组织中的角色
    pub async fn get_org_role(
        &self,
        identity_id: Uuid,
        org_id: Uuid,
    ) -> Result<Option<OrgRole>, AppError> {
        let role_str = self
            .org_membership_repo
            .get_role(identity_id, org_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;
        Ok(role_str.map(|s| OrgRole::from(s.as_str())))
    }

    /// 获取用户所在的所有组织
    pub async fn get_user_orgs(
        &self,
        identity_id: Uuid,
    ) -> Result<Vec<crate::db::repositories::org_membership::UserOrgInfo>, AppError> {
        self.org_membership_repo
            .list_user_orgs_full(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
    }

    /// 校验当前用户是否可以对指定 Skill 执行某个操作
    /// 返回 Ok(()) 表示有权限，Err 返回无权限原因描述
    pub async fn check_skill_permission(
        &self,
        identity_id: Uuid,
        skill_owner_type: &str,
        skill_owner_id: Option<Uuid>,
        skill_author_identity_id: Option<Uuid>,
        skill_status: &str,
        skill_visibility: &str,
        action: SkillAction,
    ) -> Result<(), String> {
        // 1. 超级管理员（system_role_assignments 表）拥有所有权限
        match self.is_super_admin(identity_id).await {
            Ok(true) => return Ok(()),
            Err(e) => {
                tracing::error!(
                    identity_id = %identity_id,
                    error = %e,
                    "Failed to check super_admin role, falling through to other checks"
                );
            }
            _ => {}
        }

        // 2. 系统管理员（identities.is_system_admin 字段）拥有所有权限
        match self.is_system_admin(identity_id).await {
            Ok(true) => return Ok(()),
            Err(e) => {
                tracing::error!(
                    identity_id = %identity_id,
                    error = %e,
                    "Failed to check is_system_admin, falling through to other checks"
                );
            }
            _ => {}
        }

        // 辅助：是否是 Skill 的所有者
        let is_owner = skill_owner_type == "user"
            && (skill_owner_id == Some(identity_id)
                || skill_author_identity_id == Some(identity_id));

        match action {
            SkillAction::Read => {
                // 已发布的市场 Skill 所有人可读
                if skill_status == "published" && skill_visibility == "marketplace" {
                    return Ok(());
                }
                // 所有者可读
                if is_owner {
                    return Ok(());
                }
                // 同组织成员可读
                if skill_owner_type == "organization" {
                    if let Some(org_id) = skill_owner_id {
                        match self.is_org_member(identity_id, org_id).await {
                            Ok(true) => return Ok(()),
                            Ok(false) => {}
                            Err(e) => {
                                tracing::error!(
                                    identity_id = %identity_id,
                                    org_id = %org_id,
                                    error = %e,
                                    "Failed to check org membership for Read permission"
                                );
                            }
                        }
                    }
                }
                Err("无权访问此 Skill".to_string())
            }
            SkillAction::Update | SkillAction::SubmitReview => {
                // 个人所有者可写
                if is_owner {
                    return Ok(());
                }
                // 组织 Developer 及以上可写
                if skill_owner_type == "organization" {
                    if let Some(org_id) = skill_owner_id {
                        match self.get_org_role(identity_id, org_id).await {
                            Ok(Some(role)) if role >= OrgRole::Developer => return Ok(()),
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(
                                    identity_id = %identity_id,
                                    org_id = %org_id,
                                    error = %e,
                                    "Failed to get org role for Update/SubmitReview permission"
                                );
                            }
                        }
                    }
                }
                Err("无权修改此 Skill".to_string())
            }
            SkillAction::Delete => {
                // 个人所有者可删除
                if is_owner {
                    return Ok(());
                }
                // 组织 Admin 及以上可删除
                if skill_owner_type == "organization" {
                    if let Some(org_id) = skill_owner_id {
                        match self.get_org_role(identity_id, org_id).await {
                            Ok(Some(role)) if role >= OrgRole::Admin => return Ok(()),
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(
                                    identity_id = %identity_id,
                                    org_id = %org_id,
                                    error = %e,
                                    "Failed to get org role for Delete permission"
                                );
                            }
                        }
                    }
                }
                Err("无权删除此 Skill".to_string())
            }
            SkillAction::Approve | SkillAction::Reject => {
                // 组织 Skill：不能审核自己的，需要组织内 Reviewer 及以上审核
                if skill_owner_type == "organization" {
                    if is_owner {
                        return Err("不能审核自己的 Skill".to_string());
                    }
                    if let Some(org_id) = skill_owner_id {
                        match self.get_org_role(identity_id, org_id).await {
                            Ok(Some(role)) if role >= OrgRole::Reviewer => return Ok(()),
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(
                                    identity_id = %identity_id,
                                    org_id = %org_id,
                                    error = %e,
                                    "Failed to get org role for Approve/Reject permission"
                                );
                            }
                        }
                    }
                    return Err("无权审核此 Skill，需要组织 Reviewer 及以上权限".to_string());
                }
                // 个人 Skill：所有者可直接审核
                if is_owner {
                    return Ok(());
                }
                Err("无权审核此 Skill".to_string())
            }
            SkillAction::Publish => {
                if is_owner {
                    return Ok(());
                }
                if skill_owner_type == "organization" {
                    if let Some(org_id) = skill_owner_id {
                        match self.get_org_role(identity_id, org_id).await {
                            Ok(Some(role)) if role >= OrgRole::Admin => return Ok(()),
                            Ok(_) => {}
                            Err(e) => {
                                tracing::error!(
                                    identity_id = %identity_id,
                                    org_id = %org_id,
                                    error = %e,
                                    "Failed to get org role for Publish permission"
                                );
                            }
                        }
                    }
                }
                Err("无权发布此 Skill".to_string())
            }
        }
    }

    pub async fn build_context(&self, identity_id: Uuid) -> Result<PermissionContext, AppError> {
        let system_assignments = self
            .system_role_repo
            .find_by_identity(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let system_roles: HashSet<String> = system_assignments
            .into_iter()
            .map(|a| a.role_name)
            .collect();

        let org_memberships = self
            .org_membership_repo
            .list_user_organizations(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        let org_roles: Vec<(Uuid, String)> = org_memberships
            .into_iter()
            .map(|(org_id, role)| (org_id, role))
            .collect();

        let group_roles = self
            .group_repo
            .list_user_group_memberships(identity_id)
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))?;

        Ok(PermissionContext {
            identity_id,
            system_roles,
            org_roles,
            group_roles,
        })
    }

    pub async fn has_permission(
        &self,
        ctx: &PermissionContext,
        permission_code: &str,
        resource: Option<&ResourceScope>,
    ) -> Result<bool, AppError> {
        if ctx.system_roles.contains("super_admin") {
            return Ok(true);
        }

        let mut role_entries: Vec<(String, String, Option<Uuid>)> = Vec::new();

        for role_name in &ctx.system_roles {
            role_entries.push(("system".to_string(), role_name.clone(), None));
        }

        for (org_id, role_name) in &ctx.org_roles {
            role_entries.push(("organization".to_string(), role_name.clone(), Some(*org_id)));
        }

        for (group_id, role_name) in &ctx.group_roles {
            role_entries.push(("group".to_string(), role_name.clone(), Some(*group_id)));
        }

        for (role_level, role_name, scope_id) in &role_entries {
            let perms = self
                .role_permission_repo
                .list_by_role(role_level, role_name)
                .await
                .map_err(|e| AppError::InternalError(e.to_string()))?;

            for perm in &perms {
                if perm.permission_code != permission_code {
                    continue;
                }

                if let Some(resource) = resource {
                    match perm.scope_restriction.as_str() {
                        "none" => {}
                        "own" => {
                            if let Some(author_id) = resource.author_identity_id {
                                if author_id != ctx.identity_id {
                                    continue;
                                }
                            }
                        }
                        "org" => {
                            if let Some(scope_org_id) = scope_id {
                                if let Some(resource_org_id) = resource.organization_id {
                                    if *scope_org_id != resource_org_id {
                                        continue;
                                    }
                                } else if let Some(owner_id) = resource.owner_id {
                                    if *scope_org_id != owner_id {
                                        continue;
                                    }
                                }
                            }
                        }
                        "group" => {
                            if let Some(scope_group_id) = scope_id {
                                if let Some(resource_group_id) = resource.group_id {
                                    if *scope_group_id != resource_group_id {
                                        continue;
                                    }
                                } else if let Some(owner_id) = resource.owner_id {
                                    if *scope_group_id != owner_id {
                                        continue;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if role_level == "group" {
                    if let Some(current_group_id) = scope_id {
                        let override_result = self
                            .group_perm_override_repo
                            .find_by_group_role_permission(
                                *current_group_id,
                                role_name,
                                permission_code,
                            )
                            .await
                            .map_err(|e| AppError::InternalError(e.to_string()))?;

                        match override_result {
                            Some(ov) if ov.granted => return Ok(true),
                            Some(_) => continue, // override denied: skip this role
                            None => {
                                // 无 override 记录时默认允许（向后兼容）
                                return Ok(true);
                            }
                        }
                    }
                }

                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn can_edit_skill(
        &self,
        ctx: &PermissionContext,
        skill_owner_type: &str,
        skill_owner_id: Option<Uuid>,
    ) -> Result<bool, AppError> {
        if skill_owner_type == "user" {
            return Ok(skill_owner_id == Some(ctx.identity_id));
        }

        if ctx.system_roles.contains("super_admin") {
            return Ok(true);
        }

        if skill_owner_type == "organization" {
            let resource = skill_owner_id.map(|org_id| ResourceScope {
                owner_type: Some("organization".to_string()),
                owner_id: Some(org_id),
                author_identity_id: None,
                organization_id: Some(org_id),
                group_id: None,
            });

            if self
                .has_permission(ctx, "skill:update", resource.as_ref())
                .await?
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn can_create_skill(
        &self,
        ctx: &PermissionContext,
        owner_type: &str,
        target_org_id: Option<Uuid>,
    ) -> Result<bool, AppError> {
        if owner_type == "user" {
            return Ok(true);
        }

        if owner_type == "organization" {
            if let Some(org_id) = target_org_id {
                let resource = Some(ResourceScope {
                    owner_type: Some("organization".to_string()),
                    owner_id: Some(org_id),
                    author_identity_id: None,
                    organization_id: Some(org_id),
                    group_id: None,
                });

                return self
                    .has_permission(ctx, "skill:create", resource.as_ref())
                    .await;
            }
            return Ok(false);
        }

        Ok(false)
    }
}
