//! Admin authentication helpers for the tenant-scope guard.
//!
//! See `docs/superpowers/specs/2026-06-03-tenant-scope-guard-design.md`
//! for the design intent.

use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::http_state::AppRouterState;
use crate::api::jwt::AdminUser;

/// Verify that the requesting user is allowed to access the given tenant.
/// - super_admin: always allowed
/// - user with org membership in the tenant: allowed
/// - otherwise: Forbidden
///
/// Used by every single-tenant admin handler in Tasks 6-12.
pub async fn require_tenant_access(
    state: &AppRouterState,
    user: &AdminUser,
    tenant_id: Uuid,
) -> Result<(), ApiError> {
    let belongs = state
        .permission
        .user_belongs_to_tenant(user.identity_id, tenant_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if belongs {
        Ok(())
    } else {
        Err(ApiError::Forbidden(
            "Not a member of this tenant".to_string(),
        ))
    }
}

/// Build a tenant-id filter for list endpoints. Returns
/// `(is_unrestricted, allowed_tenant_ids)`. If `is_unrestricted` is
/// true (super_admin), the caller should not apply any tenant
/// filter. If false, the caller should filter results to
/// `tenant_id = ANY(allowed_tenant_ids)` (or equivalent).
///
/// Used by every list admin handler in Tasks 6-12.
pub async fn tenant_filter_for_user(
    state: &AppRouterState,
    user: &AdminUser,
) -> Result<(bool, Vec<Uuid>), ApiError> {
    let is_super = state
        .permission
        .is_super_admin_user(user.identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    if is_super {
        return Ok((true, Vec::new()));
    }
    let tenants = state
        .permission
        .list_user_tenants(user.identity_id)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;
    Ok((false, tenants))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests for require_tenant_access and tenant_filter_for_user
    // live in tests/admin_isolation.rs (added in Task 13). Unit-testing these
    // helpers requires mocking AppRouterState / PermissionService, which the
    // project does not currently have a framework for.

    #[test]
    fn test_module_compiles() {
        let _ = std::marker::PhantomData::<AdminUser>;
    }
}
