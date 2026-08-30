-- Migration: Add tenant_creation_requests table for self-service tenant application workflow
-- This table stores tenant creation requests when TENANT_APPROVAL_REQUIRED=true

CREATE TABLE IF NOT EXISTS tenant_creation_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Applicant information
    applicant_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    applicant_name VARCHAR(255) NOT NULL,
    applicant_email VARCHAR(255) NOT NULL,
    -- Requested tenant information
    tenant_name VARCHAR(255) NOT NULL,
    tenant_slug VARCHAR(255) NOT NULL,
    -- Optional message from applicant
    message TEXT,
    -- Request status: pending | approved | rejected
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    -- Approval/rejection info
    reviewed_by UUID REFERENCES identities(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMPTZ,
    review_note TEXT,
    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- If approved, store the created tenant ID
    tenant_id UUID REFERENCES tenants(id) ON DELETE SET NULL
);

-- Indexes for common queries
CREATE INDEX idx_tenant_creation_requests_applicant ON tenant_creation_requests(applicant_id);
CREATE INDEX idx_tenant_creation_requests_status ON tenant_creation_requests(status);
CREATE INDEX idx_tenant_creation_requests_created_at ON tenant_creation_requests(created_at DESC);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_tenant_creation_requests_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Trigger to auto-update updated_at
CREATE TRIGGER trigger_tenant_creation_requests_updated_at
    BEFORE UPDATE ON tenant_creation_requests
    FOR EACH ROW
    EXECUTE FUNCTION update_tenant_creation_requests_updated_at();

-- Comments
COMMENT ON TABLE tenant_creation_requests IS 'Stores tenant creation requests for self-service workflow when approval is required';
COMMENT ON COLUMN tenant_creation_requests.applicant_id IS 'User who submitted the request';
COMMENT ON COLUMN tenant_creation_requests.tenant_name IS 'Requested tenant name';
COMMENT ON COLUMN tenant_creation_requests.tenant_slug IS 'Auto-generated or requested tenant slug';
COMMENT ON COLUMN tenant_creation_requests.message IS 'Optional message from applicant';
COMMENT ON COLUMN tenant_creation_requests.status IS 'Request status: pending, approved, rejected';
COMMENT ON COLUMN tenant_creation_requests.reviewed_by IS 'Super admin who reviewed the request';
COMMENT ON COLUMN tenant_creation_requests.tenant_id IS 'Created tenant ID after approval';
