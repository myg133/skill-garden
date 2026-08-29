-- Migration: 007_add_skill_policies
-- Description: Add skill_policies table for skill visibility control

CREATE TABLE skill_policies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    org_id UUID NOT NULL,
    skill_id VARCHAR(255) NOT NULL,
    visibility VARCHAR(50) NOT NULL DEFAULT 'org_visible',
    allowed_agents JSONB DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_skill_policies_org FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE CASCADE,
    UNIQUE(org_id, skill_id)
);

CREATE INDEX idx_skill_policies_org ON skill_policies(org_id);
CREATE INDEX idx_skill_policies_skill ON skill_policies(skill_id);
