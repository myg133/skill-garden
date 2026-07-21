-- Migration 034: Clean up legacy permissions (from migration 014)
-- Remove old pre-RBAC permission codes that have been replaced by the new permission model

-- Phase 2: Remove legacy permission codes from role_permissions table
DELETE FROM role_permissions WHERE permission_code IN (
    'skill:approve',           -- replaced by skill:approve_review
    'skill:publish',           -- replaced by skill:publish (internal) + skill:publish_to_marketplace
    'tool:read',               -- tool permissions not used
    'tool:write',              -- tool permissions not used
    'org:manage',              -- replaced by org:update, org:delete, etc.
    'org:configure',           -- replaced by org:settings_read/write
    'org:view',                -- replaced by org:read
    'member:manage',           -- replaced by org:member_invite/remove/role_assign
    'member:view',             -- replaced by org:member_read
    'tenant:manage',           -- replaced by tenant:update/delete/etc.
    'group:manage',            -- group handling is postponed
    'group:view',              -- group handling is postponed
    'audit:read',              -- replaced by audit:read_global / audit:read_org
    'api_key:manage',          -- replaced by apikey:create/read/revoke
    'skill:associate_group',   -- group Skill association postponed
    'skill:dissociate_group'   -- group Skill association postponed
);

-- Remove org:skill_publish if it still exists (replaced by skill:publish_to_marketplace)
DELETE FROM role_permissions WHERE permission_code = 'org:skill_publish';

-- Mark admin_unpublished column in skills table as deprecated (no-op, just documentation)
-- The column is kept for backward compatibility but no longer used by the application layer.
-- See migration 032 for the marketplace_status replacement.
