use std::collections::HashSet;
use uuid::Uuid;

use crate::db::repositories::group::GroupRepository;
use crate::db::repositories::{
    GroupPermissionOverrideRepository, OrgMembershipRepository, RolePermissionRepository,
    SystemRoleAssignmentRepository,
};
use crate::models::error::AppError;

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

#[derive(Clone)]
pub struct PermissionService {
    system_role_repo: SystemRoleAssignmentRepository,
    org_membership_repo: OrgMembershipRepository,
    role_permission_repo: RolePermissionRepository,
    group_perm_override_repo: GroupPermissionOverrideRepository,
    group_repo: GroupRepository,
}

impl PermissionService {
    pub fn new(
        system_role_repo: SystemRoleAssignmentRepository,
        org_membership_repo: OrgMembershipRepository,
        role_permission_repo: RolePermissionRepository,
        group_perm_override_repo: GroupPermissionOverrideRepository,
        group_repo: GroupRepository,
    ) -> Self {
        Self {
            system_role_repo,
            org_membership_repo,
            role_permission_repo,
            group_perm_override_repo,
            group_repo,
        }
    }

    pub async fn is_super_admin(&self, identity_id: Uuid) -> Result<bool, AppError> {
        self.system_role_repo
            .has_system_role(identity_id, "super_admin")
            .await
            .map_err(|e| AppError::InternalError(e.to_string()))
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
                        "none" => {},
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
                            .find_by_group_role_permission(*current_group_id, role_name, permission_code)
                            .await
                            .map_err(|e| AppError::InternalError(e.to_string()))?;

                        if let Some(ov) = override_result {
                            if !ov.granted {
                                continue;
                            }
                            return Ok(true);
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

                return self.has_permission(ctx, "skill:create", resource.as_ref()).await;
            }
            return Ok(false);
        }

        Ok(false)
    }
}