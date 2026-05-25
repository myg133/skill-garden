-- Migration: 006_add_org_tools
-- Description: Add org_tools table for organization private tools

CREATE TABLE org_tools (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tool_id VARCHAR(255) NOT NULL,
    org_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    schema JSONB NOT NULL,
    implementation JSONB NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT fk_org_tools_org FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE CASCADE,
    UNIQUE(org_id, tool_id)
);

CREATE INDEX idx_org_tools_org ON org_tools(org_id);
CREATE INDEX idx_org_tools_status ON org_tools(status);
