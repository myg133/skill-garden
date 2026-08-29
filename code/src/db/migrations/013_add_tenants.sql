-- Migration: 013_add_tenants
-- Description: Add multi-tenant support with tenants, organizations, groups
-- 运行在 012 之后

CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Tenants 表（租户/公司）
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) UNIQUE NOT NULL,
    status VARCHAR(50) DEFAULT 'active',
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_tenants_slug ON tenants(slug);
CREATE INDEX idx_tenants_status ON tenants(status);

-- Organizations 表添加多租户字段（004 创建的表只有: id, name, settings, created_at）
ALTER TABLE organizations ADD COLUMN slug VARCHAR(100);
ALTER TABLE organizations ADD COLUMN tenant_id UUID REFERENCES tenants(id) ON DELETE SET NULL;
ALTER TABLE organizations ADD COLUMN org_type VARCHAR(50) DEFAULT 'standard';
ALTER TABLE organizations ADD COLUMN description TEXT;
ALTER TABLE organizations ADD COLUMN status VARCHAR(50) DEFAULT 'active';
ALTER TABLE organizations ADD COLUMN updated_at TIMESTAMPTZ DEFAULT NOW();

-- 为 004 创建的 Marketplace Review org 添加 slug
UPDATE organizations SET slug = 'marketplace-review' WHERE name = 'Marketplace Review' AND slug IS NULL;
-- 确保 marketplace review org 没有 tenant
UPDATE organizations SET tenant_id = NULL WHERE name = 'Marketplace Review';

CREATE INDEX idx_organizations_tenant_id ON organizations(tenant_id);
CREATE INDEX idx_organizations_org_type ON organizations(org_type);
CREATE INDEX idx_organizations_slug ON organizations(slug);
CREATE INDEX idx_organizations_status ON organizations(status);

-- Groups 表
CREATE TABLE groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(100) NOT NULL,
    description TEXT,
    group_type VARCHAR(50) DEFAULT 'team',
    settings JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(organization_id, slug)
);

CREATE INDEX idx_groups_organization_id ON groups(organization_id);
CREATE INDEX idx_groups_slug ON groups(slug);
