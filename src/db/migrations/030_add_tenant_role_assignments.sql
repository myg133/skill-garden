-- Migration: 030_add_tenant_role_assignments
-- Description: Add tenant_role_assignments table for tenant-level RBAC (tenant_admin, etc.)
--              Complements system_role_assignments (system layer) and org_memberships (org layer).
--              This completes the 4-tier RBAC: system -> tenant -> organization -> group.

CREATE TABLE tenant_role_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    role_name VARCHAR(50) NOT NULL,
    assigned_by UUID REFERENCES identities(id),
    assigned_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(identity_id, tenant_id, role_name)
);

CREATE INDEX idx_tenant_role_assignments_identity ON tenant_role_assignments(identity_id);
CREATE INDEX idx_tenant_role_assignments_tenant ON tenant_role_assignments(tenant_id);
CREATE INDEX idx_tenant_role_assignments_role ON tenant_role_assignments(role_name);

-- Seed: tenant_admin role permissions (tenant-level permissions via role_permissions table)
-- These build on the tenant permissions already defined in migration 018.

INSERT INTO permissions (code, name, resource_type, action, scope) VALUES
    ('tenant:member_read', 'Read Tenant Members', 'tenant', 'read', 'none'),
    ('tenant:member_invite', 'Invite Tenant Members', 'tenant', 'invite', 'none'),
    ('tenant:member_remove', 'Remove Tenant Members', 'tenant', 'remove', 'none'),
    ('tenant:org_create', 'Create Organization in Tenant', 'org', 'create', 'none'),
    ('tenant:org_delete', 'Delete Organization in Tenant', 'org', 'delete', 'none'),
    ('tenant:billing_read', 'Read Billing Info', 'tenant', 'read', 'none'),
    ('tenant:billing_update', 'Update Billing Info', 'tenant', 'update', 'none'),
    ('tenant:settings_read', 'Read Tenant Settings', 'tenant', 'read', 'none'),
    ('tenant:settings_write', 'Write Tenant Settings', 'tenant', 'write', 'none')
ON CONFLICT (code) DO NOTHING;

-- tenant_admin tenant-level permissions
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('tenant', 'tenant_admin', 'tenant:read', 'none'),
    ('tenant', 'tenant_admin', 'tenant:update', 'none'),
    ('tenant', 'tenant_admin', 'tenant:settings_read', 'none'),
    ('tenant', 'tenant_admin', 'tenant:settings_write', 'none'),
    ('tenant', 'tenant_admin', 'tenant:sso_config', 'none'),
    ('tenant', 'tenant_admin', 'tenant:member_read', 'none'),
    ('tenant', 'tenant_admin', 'tenant:member_invite', 'none'),
    ('tenant', 'tenant_admin', 'tenant:member_remove', 'none'),
    ('tenant', 'tenant_admin', 'tenant:org_create', 'none'),
    ('tenant', 'tenant_admin', 'tenant:org_delete', 'none'),
    ('tenant', 'tenant_admin', 'tenant:billing_read', 'none'),
    ('tenant', 'tenant_admin', 'tenant:billing_update', 'none'),
    ('tenant', 'tenant_admin', 'org:read', 'tenant'),
    ('tenant', 'tenant_admin', 'org:member_read', 'tenant'),
    ('tenant', 'tenant_admin', 'skill:read', 'tenant'),
    ('tenant', 'tenant_admin', 'skill:read_content', 'tenant'),
    ('tenant', 'tenant_admin', 'skill:install', 'tenant'),
    ('tenant', 'tenant_admin', 'group:read', 'tenant'),
    ('tenant', 'tenant_admin', 'group:member_read', 'tenant')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;
