-- Migration: 014_add_identities_and_roles
-- Description: Add identities, memberships, roles and permissions
-- 运行在 013 之后

-- Identities 表（身份 - User/Agent 的统一抽象）
CREATE TABLE identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identity_type VARCHAR(50) NOT NULL,
    external_id VARCHAR(255),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255),
    avatar_url VARCHAR(500),
    status VARCHAR(50) DEFAULT 'active',
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_identities_type ON identities(identity_type);
CREATE INDEX idx_identities_external_id ON identities(external_id);
CREATE INDEX idx_identities_email ON identities(email);
CREATE INDEX idx_identities_status ON identities(status);

-- Memberships 表（组成员关系）
CREATE TABLE memberships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    group_id UUID NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(identity_id, group_id)
);

CREATE INDEX idx_memberships_identity_id ON memberships(identity_id);
CREATE INDEX idx_memberships_group_id ON memberships(group_id);

-- Organization Identities 表（组织身份关联 - 用于外部 Agent 与组织绑定）
CREATE TABLE organization_identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    permissions JSONB DEFAULT '[]',
    status VARCHAR(50) DEFAULT 'active',
    last_accessed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(organization_id, identity_id)
);

CREATE INDEX idx_org_identities_org_id ON organization_identities(organization_id);
CREATE INDEX idx_org_identities_identity_id ON organization_identities(identity_id);

-- Roles 表（角色定义）
CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    role_type VARCHAR(50) NOT NULL,
    scope_level VARCHAR(50),
    parent_role_id UUID REFERENCES roles(id),
    permissions JSONB NOT NULL DEFAULT '[]',
    description TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(name, role_type)
);

CREATE INDEX idx_roles_type ON roles(role_type);
CREATE INDEX idx_roles_scope_level ON roles(scope_level);

-- Identity Roles 表（身份角色分配）
CREATE TABLE identity_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    scope_id UUID,
    granted_by UUID REFERENCES identities(id),
    granted_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    UNIQUE(identity_id, role_id, scope_id)
);

CREATE INDEX idx_identity_roles_identity_id ON identity_roles(identity_id);
CREATE INDEX idx_identity_roles_role_id ON identity_roles(role_id);
CREATE INDEX idx_identity_roles_scope_id ON identity_roles(scope_id);

-- Permissions 表（权限点定义）
CREATE TABLE permissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(100) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    resource_type VARCHAR(100) NOT NULL,
    action VARCHAR(50) NOT NULL,
    description TEXT
);

CREATE INDEX idx_permissions_code ON permissions(code);
CREATE INDEX idx_permissions_resource_type ON permissions(resource_type);

-- 插入默认权限
INSERT INTO permissions (code, name, resource_type, action) VALUES
('skill:create', 'Create Skill', 'skill', 'create'),
('skill:read', 'Read Skill', 'skill', 'read'),
('skill:update', 'Update Skill', 'skill', 'update'),
('skill:delete', 'Delete Skill', 'skill', 'delete'),
('skill:approve', 'Approve Skill', 'skill', 'approve'),
('skill:publish', 'Publish Skill', 'skill', 'publish'),
('tool:register', 'Register Tool', 'tool', 'register'),
('tool:read', 'Read Tool', 'tool', 'read'),
('tool:update', 'Update Tool', 'tool', 'update'),
('tool:delete', 'Delete Tool', 'tool', 'delete'),
('org:manage', 'Manage Organization', 'org', 'manage'),
('org:configure', 'Configure Organization', 'org', 'configure'),
('org:view', 'View Organization', 'org', 'view'),
('member:manage', 'Manage Members', 'member', 'manage'),
('member:view', 'View Members', 'member', 'view'),
('tenant:manage', 'Manage Tenant', 'tenant', 'manage'),
('group:manage', 'Manage Group', 'group', 'manage'),
('group:view', 'View Group', 'group', 'view'),
('audit:read', 'Read Audit Logs', 'audit', 'read'),
('api_key:manage', 'Manage API Keys', 'api_key', 'manage')
ON CONFLICT (code) DO NOTHING;

-- 插入默认角色
INSERT INTO roles (name, role_type, scope_level, permissions, description) VALUES
('super_admin', 'system', 'global', '["*"]', 'Super Administrator with full access'),
('marketplace_admin', 'system', 'global', '["skill:approve", "skill:publish", "org:view", "member:view"]', 'Marketplace Administrator for skill review'),
('tenant_admin', 'tenant', 'tenant', '["tenant:manage", "org:manage", "member:manage", "skill:*", "tool:*"]', 'Tenant Administrator'),
('org_admin', 'organization', 'org', '["org:manage", "org:configure", "member:manage", "skill:*", "tool:*"]', 'Organization Administrator'),
('skill_developer', 'organization', 'org', '["skill:create", "skill:read", "skill:update", "tool:register"]', 'Skill Developer')
ON CONFLICT (name, role_type) DO NOTHING;
