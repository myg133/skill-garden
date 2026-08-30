-- Migration: Add org_join_requests table for organization join request workflow
-- This supports the SaaS scenario where users can proactively apply to join organizations

-- Create org_join_requests table
CREATE TABLE IF NOT EXISTS org_join_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending' 
        CHECK (status IN ('pending', 'approved', 'rejected')),
    message TEXT,
    reviewed_by UUID REFERENCES identities(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- Ensure only one pending request per user per org
    CONSTRAINT unique_pending_request UNIQUE (organization_id, identity_id, status)
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_org_join_requests_org ON org_join_requests(organization_id);
CREATE INDEX IF NOT EXISTS idx_org_join_requests_identity ON org_join_requests(identity_id);
CREATE INDEX IF NOT EXISTS idx_org_join_requests_status ON org_join_requests(status);
CREATE INDEX IF NOT EXISTS idx_org_join_requests_created ON org_join_requests(created_at DESC);

-- Add join_policy column to organizations table
ALTER TABLE organizations 
ADD COLUMN IF NOT EXISTS join_policy VARCHAR(20) 
NOT NULL DEFAULT 'approval_required'
CHECK (join_policy IN ('invite_only', 'approval_required', 'open'));

COMMENT ON COLUMN organizations.join_policy IS 
'Join policy: invite_only=invite only, approval_required=requires approval, open=anyone can join';

-- Create partial unique index for pending requests only (allows re-applying after rejection)
CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_pending_request 
ON org_join_requests(organization_id, identity_id) 
WHERE status = 'pending';
