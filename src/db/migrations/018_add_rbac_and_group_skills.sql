-- Migration: 018_add_rbac_and_group_skills
-- Description: Add role_permissions, group_permission_overrides, group_skills,
--              licenses tables, and extend permissions with scope column

-- ============================================================
-- 1. 扩展 permissions 表（添加 scope 列）
-- ============================================================

ALTER TABLE permissions ADD COLUMN IF NOT EXISTS scope VARCHAR(50) DEFAULT 'global';

-- ============================================================
-- 2. 创建 role_permissions 表（角色-权限绑定）
-- ============================================================

CREATE TABLE role_permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    role_level VARCHAR(50) NOT NULL,
    role_name VARCHAR(50) NOT NULL,
    permission_code VARCHAR(100) NOT NULL REFERENCES permissions(code) ON DELETE CASCADE,
    scope_restriction VARCHAR(50) DEFAULT 'none',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(role_level, role_name, permission_code)
);

CREATE INDEX idx_role_perms_role ON role_permissions(role_level, role_name);
CREATE INDEX idx_role_perms_permission ON role_permissions(permission_code);

-- ============================================================
-- 3. 创建 group_permission_overrides 表（组级权限覆盖）
-- ============================================================

CREATE TABLE group_permission_overrides (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    role_name VARCHAR(50) NOT NULL,
    permission_code VARCHAR(100) NOT NULL REFERENCES permissions(code) ON DELETE CASCADE,
    granted BOOLEAN NOT NULL DEFAULT TRUE,
    created_by UUID REFERENCES identities(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(group_id, role_name, permission_code)
);

CREATE INDEX idx_group_perm_overrides_group ON group_permission_overrides(group_id);
CREATE INDEX idx_group_perm_overrides_role ON group_permission_overrides(group_id, role_name);

-- ============================================================
-- 4. 创建 group_skills 表（Group-Skill 关联）
-- ============================================================

CREATE TABLE group_skills (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    skill_id VARCHAR(255) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    added_by UUID REFERENCES identities(id),
    added_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(group_id, skill_id)
);

CREATE INDEX idx_group_skills_group ON group_skills(group_id);
CREATE INDEX idx_group_skills_skill ON group_skills(skill_id);

-- ============================================================
-- 5. 创建 licenses 表（许可证/计费管理）
-- ============================================================

CREATE TABLE licenses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    license_key VARCHAR(255) UNIQUE NOT NULL,
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    plan VARCHAR(50) NOT NULL DEFAULT 'free',
    max_users INTEGER NOT NULL DEFAULT 10,
    max_organizations INTEGER NOT NULL DEFAULT 5,
    max_skills INTEGER NOT NULL DEFAULT 100,
    features JSONB DEFAULT '{}',
    expires_at TIMESTAMPTZ,
    status VARCHAR(50) DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_licenses_tenant ON licenses(tenant_id);
CREATE INDEX idx_licenses_key ON licenses(license_key);
CREATE INDEX idx_licenses_status ON licenses(status);

-- ============================================================
-- 6. 扩展 agents 表（添加 identity_id 引用）
-- ============================================================

ALTER TABLE agents ADD COLUMN IF NOT EXISTS identity_id UUID REFERENCES identities(id);

-- ============================================================
-- 7. 插入默认角色-权限绑定数据
-- ============================================================

-- 组织级角色绑定
-- Ensure all permission codes referenced below exist in the permissions table
INSERT INTO permissions (code, name, resource_type, action, scope) VALUES
    ('org:read', 'Read Organization', 'org', 'read', 'global'),
    ('org:update', 'Update Organization', 'org', 'update', 'global'),
    ('org:delete', 'Delete Organization', 'org', 'delete', 'global'),
    ('org:transfer', 'Transfer Organization', 'org', 'manage', 'global'),
    ('org:settings_read', 'Read Organization Settings', 'org', 'read', 'global'),
    ('org:settings_write', 'Write Organization Settings', 'org', 'write', 'global'),
    ('org:member_read', 'Read Organization Members', 'member', 'read', 'global'),
    ('org:member_invite', 'Invite Organization Members', 'member', 'invite', 'global'),
    ('org:member_remove', 'Remove Organization Members', 'member', 'remove', 'global'),
    ('org:member_role_assign', 'Assign Organization Member Role', 'member', 'assign_role', 'global'),
    ('org:member_suspend', 'Suspend Organization Member', 'member', 'suspend', 'global'),
    ('org:skill_transfer', 'Transfer Organization Skill', 'org', 'manage', 'global'),
    ('skill:create', 'Create Skill', 'skill', 'create', 'org'),
    ('skill:read', 'Read Skill', 'skill', 'read', 'org'),
    ('skill:read_content', 'Read Skill Content', 'skill', 'read', 'org'),
    ('skill:update', 'Update Skill', 'skill', 'update', 'org'),
    ('skill:delete', 'Delete Skill', 'skill', 'delete', 'org'),
    ('skill:install', 'Install Skill', 'skill', 'install', 'org'),
    ('skill:version_create', 'Create Skill Version', 'skill', 'create', 'org'),
    ('skill:version_rollback', 'Rollback Skill Version', 'skill', 'rollback', 'org'),
    ('skill:submit_review', 'Submit Skill Review', 'skill', 'submit', 'org'),
    ('skill:approve_review', 'Approve Skill Review', 'skill', 'approve', 'org'),
    ('skill:reject_review', 'Reject Skill Review', 'skill', 'reject', 'org'),
    ('skill:change_visibility', 'Change Skill Visibility', 'skill', 'update', 'org'),
    ('skill:associate_group', 'Associate Skill with Group', 'skill', 'associate', 'org'),
    ('skill:dissociate_group', 'Dissociate Skill from Group', 'skill', 'dissociate', 'org'),
    ('skill:fork', 'Fork Skill', 'skill', 'fork', 'org'),
    ('group:create', 'Create Group', 'group', 'create', 'org'),
    ('group:read', 'Read Group', 'group', 'read', 'org'),
    ('group:update', 'Update Group', 'group', 'update', 'org'),
    ('group:delete', 'Delete Group', 'group', 'delete', 'org'),
    ('group:member_read', 'Read Group Members', 'group', 'read', 'org'),
    ('group:member_add', 'Add Group Member', 'group', 'add', 'org'),
    ('group:member_remove', 'Remove Group Member', 'group', 'remove', 'org'),
    ('group:member_role_assign', 'Assign Group Member Role', 'group', 'assign_role', 'org'),
    ('group:permission_override', 'Override Group Permission', 'group', 'manage', 'org'),
    ('apikey:create', 'Create API Key', 'api_key', 'create', 'org'),
    ('apikey:read', 'Read API Key', 'api_key', 'read', 'org'),
    ('apikey:revoke', 'Revoke API Key', 'api_key', 'revoke', 'org'),
    ('apikey:scope_set', 'Set API Key Scope', 'api_key', 'update', 'org'),
    ('apikey:rate_limit_set', 'Set API Key Rate Limit', 'api_key', 'update', 'org'),
    ('profile:read', 'Read Profile', 'profile', 'read', 'org'),
    ('profile:update', 'Update Profile', 'profile', 'update', 'org'),
    ('profile:delete', 'Delete Profile', 'profile', 'delete', 'org'),
    ('audit:read_org', 'Read Organization Audit', 'audit', 'read', 'org'),
    ('tenant:create', 'Create Tenant', 'tenant', 'create', 'none'),
    ('tenant:read', 'Read Tenant', 'tenant', 'read', 'none'),
    ('tenant:update', 'Update Tenant', 'tenant', 'update', 'none'),
    ('tenant:delete', 'Delete Tenant', 'tenant', 'delete', 'none'),
    ('tenant:sso_config', 'Configure SSO', 'tenant', 'manage', 'none'),
    ('marketplace:feature', 'Feature in Marketplace', 'marketplace', 'feature', 'none'),
    ('marketplace:unfeature', 'Unfeature from Marketplace', 'marketplace', 'unfeature', 'none'),
    ('marketplace:delist', 'Delist from Marketplace', 'marketplace', 'delist', 'none'),
    ('audit:read_global', 'Read Global Audit', 'audit', 'read', 'none')
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    -- owner 拥有所有组织级权限
    ('organization', 'owner', 'org:read', 'none'),
    ('organization', 'owner', 'org:update', 'none'),
    ('organization', 'owner', 'org:delete', 'none'),
    ('organization', 'owner', 'org:transfer', 'none'),
    ('organization', 'owner', 'org:settings_read', 'none'),
    ('organization', 'owner', 'org:settings_write', 'none'),
    ('organization', 'owner', 'org:member_read', 'none'),
    ('organization', 'owner', 'org:member_invite', 'none'),
    ('organization', 'owner', 'org:member_remove', 'none'),
    ('organization', 'owner', 'org:member_role_assign', 'none'),
    ('organization', 'owner', 'org:member_suspend', 'none'),
    ('organization', 'owner', 'org:skill_transfer', 'none'),
    ('organization', 'owner', 'skill:create', 'org'),
    ('organization', 'owner', 'skill:read', 'org'),
    ('organization', 'owner', 'skill:read_content', 'org'),
    ('organization', 'owner', 'skill:update', 'org'),
    ('organization', 'owner', 'skill:delete', 'org'),
    ('organization', 'owner', 'skill:install', 'org'),
    ('organization', 'owner', 'skill:version_create', 'org'),
    ('organization', 'owner', 'skill:version_rollback', 'org'),
    ('organization', 'owner', 'skill:submit_review', 'org'),
    ('organization', 'owner', 'skill:approve_review', 'org'),
    ('organization', 'owner', 'skill:reject_review', 'org'),
    ('organization', 'owner', 'skill:change_visibility', 'org'),
    ('organization', 'owner', 'skill:associate_group', 'org'),
    ('organization', 'owner', 'skill:dissociate_group', 'org'),
    ('organization', 'owner', 'skill:fork', 'org'),
    ('organization', 'owner', 'group:create', 'org'),
    ('organization', 'owner', 'group:read', 'org'),
    ('organization', 'owner', 'group:update', 'org'),
    ('organization', 'owner', 'group:delete', 'org'),
    ('organization', 'owner', 'group:member_read', 'org'),
    ('organization', 'owner', 'group:member_add', 'org'),
    ('organization', 'owner', 'group:member_remove', 'org'),
    ('organization', 'owner', 'group:member_role_assign', 'org'),
    ('organization', 'owner', 'group:permission_override', 'org'),
    ('organization', 'owner', 'apikey:create', 'org'),
    ('organization', 'owner', 'apikey:read', 'org'),
    ('organization', 'owner', 'apikey:revoke', 'org'),
    ('organization', 'owner', 'apikey:scope_set', 'org'),
    ('organization', 'owner', 'apikey:rate_limit_set', 'org'),
    ('organization', 'owner', 'profile:read', 'org'),
    ('organization', 'owner', 'profile:update', 'org'),
    ('organization', 'owner', 'profile:delete', 'org'),
    ('organization', 'owner', 'audit:read_org', 'org'),

    -- admin
    ('organization', 'admin', 'org:read', 'none'),
    ('organization', 'admin', 'org:update', 'none'),
    ('organization', 'admin', 'org:settings_read', 'none'),
    ('organization', 'admin', 'org:settings_write', 'none'),
    ('organization', 'admin', 'org:member_read', 'none'),
    ('organization', 'admin', 'org:member_invite', 'none'),
    ('organization', 'admin', 'org:member_remove', 'none'),
    ('organization', 'admin', 'org:member_role_assign', 'none'),
    ('organization', 'admin', 'org:member_suspend', 'none'),
    ('organization', 'admin', 'org:skill_transfer', 'none'),
    ('organization', 'admin', 'skill:create', 'org'),
    ('organization', 'admin', 'skill:read', 'org'),
    ('organization', 'admin', 'skill:read_content', 'org'),
    ('organization', 'admin', 'skill:update', 'org'),
    ('organization', 'admin', 'skill:delete', 'org'),
    ('organization', 'admin', 'skill:install', 'org'),
    ('organization', 'admin', 'skill:version_create', 'org'),
    ('organization', 'admin', 'skill:version_rollback', 'org'),
    ('organization', 'admin', 'skill:submit_review', 'org'),
    ('organization', 'admin', 'skill:approve_review', 'org'),
    ('organization', 'admin', 'skill:reject_review', 'org'),
    ('organization', 'admin', 'skill:change_visibility', 'org'),
    ('organization', 'admin', 'skill:associate_group', 'org'),
    ('organization', 'admin', 'skill:dissociate_group', 'org'),
    ('organization', 'admin', 'skill:fork', 'org'),
    ('organization', 'admin', 'group:create', 'org'),
    ('organization', 'admin', 'group:read', 'org'),
    ('organization', 'admin', 'group:update', 'org'),
    ('organization', 'admin', 'group:delete', 'org'),
    ('organization', 'admin', 'group:member_read', 'org'),
    ('organization', 'admin', 'group:member_add', 'org'),
    ('organization', 'admin', 'group:member_remove', 'org'),
    ('organization', 'admin', 'group:member_role_assign', 'org'),
    ('organization', 'admin', 'group:permission_override', 'org'),
    ('organization', 'admin', 'apikey:create', 'org'),
    ('organization', 'admin', 'apikey:read', 'org'),
    ('organization', 'admin', 'apikey:revoke', 'org'),
    ('organization', 'admin', 'apikey:scope_set', 'org'),
    ('organization', 'admin', 'apikey:rate_limit_set', 'org'),
    ('organization', 'admin', 'profile:read', 'org'),
    ('organization', 'admin', 'profile:update', 'org'),
    ('organization', 'admin', 'profile:delete', 'org'),
    ('organization', 'admin', 'audit:read_org', 'org'),

    -- reviewer
    ('organization', 'reviewer', 'org:read', 'org'),
    ('organization', 'reviewer', 'org:member_read', 'org'),
    ('organization', 'reviewer', 'skill:read', 'org'),
    ('organization', 'reviewer', 'skill:read_content', 'org'),
    ('organization', 'reviewer', 'skill:install', 'org'),
    ('organization', 'reviewer', 'skill:submit_review', 'org'),
    ('organization', 'reviewer', 'skill:approve_review', 'org'),
    ('organization', 'reviewer', 'skill:reject_review', 'org'),
    ('organization', 'reviewer', 'skill:fork', 'org'),
    ('organization', 'reviewer', 'group:read', 'org'),
    ('organization', 'reviewer', 'group:member_read', 'org'),
    ('organization', 'reviewer', 'apikey:create', 'org'),
    ('organization', 'reviewer', 'apikey:read', 'org'),
    ('organization', 'reviewer', 'apikey:revoke', 'org'),
    ('organization', 'reviewer', 'profile:read', 'org'),
    ('organization', 'reviewer', 'profile:update', 'org'),
    ('organization', 'reviewer', 'profile:delete', 'org'),

    -- developer
    ('organization', 'developer', 'org:read', 'org'),
    ('organization', 'developer', 'org:member_read', 'org'),
    ('organization', 'developer', 'skill:create', 'org'),
    ('organization', 'developer', 'skill:read', 'org'),
    ('organization', 'developer', 'skill:read_content', 'org'),
    ('organization', 'developer', 'skill:install', 'org'),
    ('organization', 'developer', 'skill:submit_review', 'org'),
    ('organization', 'developer', 'skill:associate_group', 'own'),
    ('organization', 'developer', 'skill:dissociate_group', 'own'),
    ('organization', 'developer', 'skill:fork', 'org'),
    ('organization', 'developer', 'group:read', 'org'),
    ('organization', 'developer', 'group:member_read', 'org'),
    ('organization', 'developer', 'apikey:create', 'org'),
    ('organization', 'developer', 'apikey:read', 'org'),
    ('organization', 'developer', 'apikey:revoke', 'org'),
    ('organization', 'developer', 'profile:read', 'org'),
    ('organization', 'developer', 'profile:update', 'org'),
    ('organization', 'developer', 'profile:delete', 'org'),

    -- member
    ('organization', 'member', 'org:read', 'org'),
    ('organization', 'member', 'org:member_read', 'org'),
    ('organization', 'member', 'skill:read', 'org'),
    ('organization', 'member', 'skill:read_content', 'org'),
    ('organization', 'member', 'skill:install', 'org'),
    ('organization', 'member', 'skill:fork', 'org'),
    ('organization', 'member', 'group:read', 'org'),
    ('organization', 'member', 'group:member_read', 'org'),
    ('organization', 'member', 'apikey:create', 'org'),
    ('organization', 'member', 'apikey:read', 'org'),
    ('organization', 'member', 'apikey:revoke', 'org'),
    ('organization', 'member', 'profile:read', 'org'),
    ('organization', 'member', 'profile:update', 'org'),
    ('organization', 'member', 'profile:delete', 'org')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- Group 级角色绑定
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('group', 'lead', 'group:read', 'group'),
    ('group', 'lead', 'group:update', 'group'),
    ('group', 'lead', 'group:delete', 'group'),
    ('group', 'lead', 'group:member_read', 'group'),
    ('group', 'lead', 'group:member_add', 'group'),
    ('group', 'lead', 'group:member_remove', 'group'),
    ('group', 'lead', 'group:member_role_assign', 'group'),
    ('group', 'lead', 'skill:read', 'group'),
    ('group', 'lead', 'skill:read_content', 'group'),
    ('group', 'lead', 'skill:update', 'group'),
    ('group', 'lead', 'skill:delete', 'group'),
    ('group', 'lead', 'skill:install', 'group'),
    ('group', 'lead', 'skill:version_create', 'group'),
    ('group', 'lead', 'skill:version_rollback', 'group'),
    ('group', 'lead', 'skill:submit_review', 'group'),
    ('group', 'lead', 'skill:approve_review', 'group'),
    ('group', 'lead', 'skill:reject_review', 'group'),
    ('group', 'lead', 'skill:change_visibility', 'group'),
    ('group', 'lead', 'skill:associate_group', 'group'),
    ('group', 'lead', 'skill:dissociate_group', 'group'),
    ('group', 'lead', 'skill:fork', 'group'),

    ('group', 'member', 'group:read', 'group'),
    ('group', 'member', 'group:member_read', 'group'),
    ('group', 'member', 'skill:read', 'group'),
    ('group', 'member', 'skill:read_content', 'group'),
    ('group', 'member', 'skill:update', 'group'),
    ('group', 'member', 'skill:install', 'group'),
    ('group', 'member', 'skill:version_create', 'group'),
    ('group', 'member', 'skill:submit_review', 'group'),
    ('group', 'member', 'skill:fork', 'group')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- 系统级角色绑定
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('system', 'super_admin', 'tenant:create', 'none'),
    ('system', 'super_admin', 'tenant:read', 'none'),
    ('system', 'super_admin', 'tenant:update', 'none'),
    ('system', 'super_admin', 'tenant:delete', 'none'),
    ('system', 'super_admin', 'tenant:sso_config', 'none'),
    ('system', 'super_admin', 'marketplace:feature', 'none'),
    ('system', 'super_admin', 'marketplace:unfeature', 'none'),
    ('system', 'super_admin', 'marketplace:delist', 'none'),
    ('system', 'super_admin', 'audit:read_global', 'none'),
    ('system', 'super_admin', 'skill:read', 'global'),
    ('system', 'super_admin', 'skill:read_content', 'global'),
    ('system', 'super_admin', 'skill:install', 'global'),
    ('system', 'super_admin', 'skill:fork', 'global'),
    ('system', 'super_admin', 'skill:approve_review', 'global'),
    ('system', 'super_admin', 'skill:reject_review', 'global'),
    ('system', 'super_admin', 'org:read', 'global'),
    ('system', 'super_admin', 'org:member_read', 'global'),

    ('system', 'marketplace_admin', 'tenant:read', 'none'),
    ('system', 'marketplace_admin', 'marketplace:feature', 'none'),
    ('system', 'marketplace_admin', 'marketplace:unfeature', 'none'),
    ('system', 'marketplace_admin', 'marketplace:delist', 'none'),
    ('system', 'marketplace_admin', 'skill:read', 'global'),
    ('system', 'marketplace_admin', 'skill:read_content', 'global'),
    ('system', 'marketplace_admin', 'skill:install', 'global'),
    ('system', 'marketplace_admin', 'skill:fork', 'global'),
    ('system', 'marketplace_admin', 'skill:approve_review', 'global'),
    ('system', 'marketplace_admin', 'skill:reject_review', 'global')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- 个人级角色绑定
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('personal', 'user', 'skill:create', 'own'),
    ('personal', 'user', 'skill:read', 'own'),
    ('personal', 'user', 'skill:read_content', 'own'),
    ('personal', 'user', 'skill:update', 'own'),
    ('personal', 'user', 'skill:delete', 'own'),
    ('personal', 'user', 'skill:install', 'own'),
    ('personal', 'user', 'skill:version_create', 'own'),
    ('personal', 'user', 'skill:version_rollback', 'own'),
    ('personal', 'user', 'skill:submit_review', 'own'),
    ('personal', 'user', 'skill:change_visibility', 'own'),
    ('personal', 'user', 'skill:fork', 'own'),
    ('personal', 'user', 'apikey:create', 'own'),
    ('personal', 'user', 'apikey:read', 'own'),
    ('personal', 'user', 'apikey:revoke', 'own'),
    ('personal', 'user', 'profile:read', 'own'),
    ('personal', 'user', 'profile:update', 'own'),
    ('personal', 'user', 'profile:delete', 'own')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;