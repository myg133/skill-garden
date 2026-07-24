-- 038: Add is_current flag to skills table + tenant_admin role_permissions
-- Part 1: is_current flag for skills version management
--   is_current = true means this version is visible in lists / installable.
--   Rules:
--     1. New upload (pending_review): new=true, old published stays=true
--     2. Approved + published: old becomes false, new stays true
--     3. Rejected: is_current stays false

ALTER TABLE skills ADD COLUMN IF NOT EXISTS is_current BOOLEAN NOT NULL DEFAULT true;

WITH latest AS (
    SELECT DISTINCT ON (name) id, name
    FROM skills
    ORDER BY name, created_at DESC
)
UPDATE skills s
SET is_current = (s.id = latest.id)
FROM latest
WHERE s.name = latest.name;

CREATE INDEX IF NOT EXISTS idx_skills_name_current ON skills(name, is_current) WHERE is_current = true;

-- Part 2: tenant_admin role_permissions
--   tenant_admin has full management rights over all orgs in its tenant

INSERT INTO permissions (code, name, resource_type, action, scope) VALUES
    ('tenant:org_manage', 'Manage Organizations in Tenant', 'org', 'manage', 'tenant'),
    ('tenant:member_manage', 'Manage Members in Tenant', 'member', 'manage', 'tenant'),
    ('tenant:skill_manage', 'Manage Skills in Tenant', 'skill', 'manage', 'tenant'),
    ('tenant:group_manage', 'Manage Groups in Tenant', 'group', 'manage', 'tenant')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('tenant', 'tenant_admin', 'org:read', 'none'),
    ('tenant', 'tenant_admin', 'org:update', 'none'),
    ('tenant', 'tenant_admin', 'org:delete', 'none'),
    ('tenant', 'tenant_admin', 'org:transfer', 'none'),
    ('tenant', 'tenant_admin', 'org:member_read', 'none'),
    ('tenant', 'tenant_admin', 'org:member_invite', 'none'),
    ('tenant', 'tenant_admin', 'org:member_remove', 'none'),
    ('tenant', 'tenant_admin', 'org:member_role_assign', 'none'),
    ('tenant', 'tenant_admin', 'org:member_suspend', 'none'),
    ('tenant', 'tenant_admin', 'org:skill_transfer', 'none'),
    ('tenant', 'tenant_admin', 'skill:create', 'none'),
    ('tenant', 'tenant_admin', 'skill:read', 'none'),
    ('tenant', 'tenant_admin', 'skill:update', 'none'),
    ('tenant', 'tenant_admin', 'skill:delete', 'none'),
    ('tenant', 'tenant_admin', 'skill:install', 'none'),
    ('tenant', 'tenant_admin', 'skill:fork', 'none'),
    ('tenant', 'tenant_admin', 'skill:submit_review', 'none'),
    ('tenant', 'tenant_admin', 'skill:approve_review', 'none'),
    ('tenant', 'tenant_admin', 'skill:reject_review', 'none'),
    ('tenant', 'tenant_admin', 'skill:publish', 'none'),
    ('tenant', 'tenant_admin', 'skill:publish_to_marketplace', 'none'),
    ('tenant', 'tenant_admin', 'group:read', 'none'),
    ('tenant', 'tenant_admin', 'group:create', 'none'),
    ('tenant', 'tenant_admin', 'group:update', 'none'),
    ('tenant', 'tenant_admin', 'group:delete', 'none'),
    ('tenant', 'tenant_admin', 'group:member_add', 'none'),
    ('tenant', 'tenant_admin', 'group:member_remove', 'none'),
    ('tenant', 'tenant_admin', 'group:member_role_assign', 'none'),
    ('tenant', 'tenant_admin', 'tenant:read', 'none'),
    ('tenant', 'tenant_admin', 'tenant:update', 'none'),
    ('tenant', 'tenant_admin', 'tenant:org_create', 'none'),
    ('tenant', 'tenant_admin', 'tenant:org_manage', 'none'),
    ('tenant', 'tenant_admin', 'tenant:member_manage', 'none'),
    ('tenant', 'tenant_admin', 'tenant:skill_manage', 'none'),
    ('tenant', 'tenant_admin', 'tenant:group_manage', 'none')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;
