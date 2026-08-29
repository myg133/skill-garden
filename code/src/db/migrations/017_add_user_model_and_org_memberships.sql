-- Migration: 017_add_user_model_and_org_memberships
-- Description: Extend identities to match user model, add org_memberships,
--              add role to memberships, extend tenants and organizations
-- 运行在 016 之后

-- ============================================================
-- 1. 扩展 identities 表，使其匹配设计文档的 users 模型
-- ============================================================

ALTER TABLE identities ADD COLUMN username VARCHAR(255);
ALTER TABLE identities ADD COLUMN display_name VARCHAR(255);
ALTER TABLE identities ADD COLUMN password_hash VARCHAR(255);

-- 用现有 name 回填 username（仅对 username 为 NULL 的行）
UPDATE identities SET username = name WHERE username IS NULL;

-- 添加唯一约束
ALTER TABLE identities ADD CONSTRAINT identities_username_unique UNIQUE (username);
ALTER TABLE identities ADD CONSTRAINT identities_email_unique UNIQUE (email);

CREATE INDEX idx_identities_username ON identities(username);

-- ============================================================
-- 2. 创建 org_memberships 表（用户-组织多对多关系）
-- ============================================================

CREATE TABLE org_memberships (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    role VARCHAR(50) NOT NULL DEFAULT 'member',
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    invited_by UUID REFERENCES identities(id),
    UNIQUE(identity_id, organization_id)
);

CREATE INDEX idx_org_memberships_identity ON org_memberships(identity_id);
CREATE INDEX idx_org_memberships_org ON org_memberships(organization_id);
CREATE INDEX idx_org_memberships_role ON org_memberships(role);

-- ============================================================
-- 3. 扩展 memberships 表（添加 role 字段，作为 group_memberships）
-- ============================================================

ALTER TABLE memberships ADD COLUMN role VARCHAR(50) NOT NULL DEFAULT 'member';

-- ============================================================
-- 4. 扩展 tenants 表
-- ============================================================

ALTER TABLE tenants ADD COLUMN billing_plan VARCHAR(50) DEFAULT 'free';
ALTER TABLE tenants ADD COLUMN sso_config JSONB DEFAULT NULL;
ALTER TABLE tenants ADD COLUMN created_by UUID REFERENCES identities(id);

-- ============================================================
-- 5. 扩展 organizations 表
-- ============================================================

ALTER TABLE organizations ADD COLUMN display_name VARCHAR(255);
ALTER TABLE organizations ADD COLUMN avatar_url VARCHAR(500);
ALTER TABLE organizations ADD COLUMN visibility VARCHAR(50) DEFAULT 'public';

-- 用 name 回填 display_name
UPDATE organizations SET display_name = name WHERE display_name IS NULL;