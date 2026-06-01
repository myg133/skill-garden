-- Migration: 015_add_api_keys_and_audit
-- Description: Add API keys, new audit logs, skill evaluations and review fields
-- 运行在 014 之后

-- api_keys 表（用于外部 Agent 接入）
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    key_hash VARCHAR(255) NOT NULL,
    key_prefix VARCHAR(20) NOT NULL,
    name VARCHAR(100),
    scopes JSONB DEFAULT '[]',
    rate_limit INTEGER DEFAULT 1000,
    status VARCHAR(50) DEFAULT 'active',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);

CREATE INDEX idx_api_keys_identity_id ON api_keys(identity_id);
CREATE INDEX idx_api_keys_organization_id ON api_keys(organization_id);
CREATE INDEX idx_api_keys_key_prefix ON api_keys(key_prefix);
CREATE INDEX idx_api_keys_status ON api_keys(status);

-- audit_log_entries 表（新结构，001 的 audit_logs 保留）
CREATE TABLE audit_log_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE SET NULL,
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    identity_id UUID NOT NULL REFERENCES identities(id),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(100),
    resource_id UUID,
    details JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_audit_log_entries_tenant_id ON audit_log_entries(tenant_id);
CREATE INDEX idx_audit_log_entries_organization_id ON audit_log_entries(organization_id);
CREATE INDEX idx_audit_log_entries_identity_id ON audit_log_entries(identity_id);
CREATE INDEX idx_audit_log_entries_action ON audit_log_entries(action);
CREATE INDEX idx_audit_log_entries_resource ON audit_log_entries(resource_type, resource_id);
CREATE INDEX idx_audit_log_entries_created_at ON audit_log_entries(created_at DESC);

-- skill_evaluations 表（新结构，与 001 的 evaluations 不同）
CREATE TABLE skill_evaluations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id VARCHAR(255) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    evaluator_identity_id UUID NOT NULL REFERENCES identities(id),
    evaluation_type VARCHAR(50) NOT NULL,
    score DECIMAL,
    results JSONB NOT NULL,
    status VARCHAR(50) DEFAULT 'completed',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_skill_evaluations_skill_id ON skill_evaluations(skill_id);
CREATE INDEX idx_skill_evaluations_evaluator_id ON skill_evaluations(evaluator_identity_id);
CREATE INDEX idx_skill_evaluations_type ON skill_evaluations(evaluation_type);
CREATE INDEX idx_skill_evaluations_created_at ON skill_evaluations(created_at DESC);

-- skills 表添加作者和审核字段（008, 011 已添加一些字段）
ALTER TABLE skills ADD COLUMN author_identity_id UUID REFERENCES identities(id);
ALTER TABLE skills ADD COLUMN owner_type VARCHAR(50) DEFAULT 'identity';
ALTER TABLE skills ADD COLUMN owner_id UUID;
ALTER TABLE skills ADD COLUMN review_status VARCHAR(50) DEFAULT 'approved';
ALTER TABLE skills ADD COLUMN reviewed_by UUID REFERENCES identities(id);
ALTER TABLE skills ADD COLUMN reviewed_at TIMESTAMPTZ;
ALTER TABLE skills ADD COLUMN review_comment TEXT;

CREATE INDEX idx_skills_author_identity_id ON skills(author_identity_id);
CREATE INDEX idx_skills_owner ON skills(owner_type, owner_id);
CREATE INDEX idx_skills_review_status ON skills(review_status);
