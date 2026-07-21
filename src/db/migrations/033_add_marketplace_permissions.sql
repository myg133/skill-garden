-- Migration: 033_add_marketplace_permissions
-- Description: Add new permission codes for dual-track marketplace model.
--              Phase 0 of permission-audit refactoring.
--              skill:publish = internal publish only (status → published)
--              skill:publish_to_marketplace = submit to marketplace review
--              marketplace:review_* = marketplace review queue permissions
--              marketplace:relist = relist a delisted skill

-- ============================================================
-- 1. Add new permission codes
-- ============================================================
INSERT INTO permissions (code, name, resource_type, action, scope) VALUES
    ('skill:publish', 'Publish Skill Internally', 'skill', 'publish', 'org'),
    ('skill:publish_to_marketplace', 'Submit to Marketplace', 'skill', 'publish', 'none'),
    ('marketplace:review_approve', 'Approve Marketplace Review', 'marketplace', 'approve', 'none'),
    ('marketplace:review_reject', 'Reject Marketplace Review', 'marketplace', 'reject', 'none'),
    ('marketplace:relist', 'Relist Marketplace Skill', 'marketplace', 'relist', 'none'),
    ('system:role_assign', 'Assign System Roles', 'system', 'assign_role', 'none'),
    ('tenant:role_assign', 'Assign Tenant Roles', 'tenant', 'assign_role', 'none'),
    ('marketplace:role_assign', 'Assign Marketplace Roles', 'marketplace', 'assign_role', 'none')
ON CONFLICT (code) DO NOTHING;

-- ============================================================
-- 2. Grant skill:publish to all roles that had implicit publish ability
-- ============================================================
-- Organization roles: owner, admin can publish internally
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('organization', 'owner', 'skill:publish', 'org'),
    ('organization', 'admin', 'skill:publish', 'org')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- Personal user: can publish own skills
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('personal', 'user', 'skill:publish', 'own')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- Group lead can publish group skills
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('group', 'lead', 'skill:publish', 'group')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- ============================================================
-- 3. Grant skill:publish_to_marketplace
--    - Org owner/admin can submit org skills to marketplace
--    - Personal user can submit own skills to marketplace
-- ============================================================
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('organization', 'owner', 'skill:publish_to_marketplace', 'org'),
    ('organization', 'admin', 'skill:publish_to_marketplace', 'org'),
    ('personal', 'user', 'skill:publish_to_marketplace', 'own')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- ============================================================
-- 4. Grant marketplace:review_* permissions
--    super_admin and marketplace_admin get review permissions
-- ============================================================
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('system', 'super_admin', 'marketplace:review_approve', 'none'),
    ('system', 'super_admin', 'marketplace:review_reject', 'none'),
    ('system', 'marketplace_admin', 'marketplace:review_approve', 'none'),
    ('system', 'marketplace_admin', 'marketplace:review_reject', 'none')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- ============================================================
-- 5. Grant marketplace:relist to super_admin and marketplace_admin
-- ============================================================
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('system', 'super_admin', 'marketplace:relist', 'none'),
    ('system', 'marketplace_admin', 'marketplace:relist', 'none')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- ============================================================
-- 6. Grant management role permissions
-- ============================================================
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('system', 'super_admin', 'system:role_assign', 'none'),
    ('system', 'super_admin', 'tenant:role_assign', 'none'),
    ('system', 'super_admin', 'marketplace:role_assign', 'none'),
    ('system', 'marketplace_admin', 'marketplace:role_assign', 'none')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- ============================================================
-- 7. Create marketplace_reviewer system role if not exists
--    (role definition is implicit via permission grants, no table needed)
-- ============================================================

-- 8. Grant marketplace_reviewer permissions
--    reviewer 可审核/下架，但不可精选推荐 (feature/unfeature)
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('system', 'marketplace_reviewer', 'marketplace:review_approve', 'none'),
    ('system', 'marketplace_reviewer', 'marketplace:review_reject', 'none'),
    ('system', 'marketplace_reviewer', 'marketplace:delist', 'none'),
    ('system', 'marketplace_reviewer', 'marketplace:manage', 'none'),
    ('system', 'marketplace_reviewer', 'skill:read', 'global'),
    ('system', 'marketplace_reviewer', 'skill:read_content', 'global')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- ============================================================
-- 9. Grant tenant:role_assign to tenant_admin
-- ============================================================
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('tenant', 'tenant_admin', 'tenant:role_assign', 'none')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- ============================================================
-- 10. Supplement developer role with missing permissions
--     (skill:update, skill:delete, skill:version_create — own scope)
-- ============================================================
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('organization', 'developer', 'skill:update', 'own'),
    ('organization', 'developer', 'skill:delete', 'own'),
    ('organization', 'developer', 'skill:version_create', 'own')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;
