-- Migration: Add tenant_creation_requests table for self-service tenant application workflow

CREATE TABLE tenant_creation_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    applicant_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    applicant_name VARCHAR(255) NOT NULL,
    applicant_email VARCHAR(255) NOT NULL,
    tenant_name VARCHAR(255) NOT NULL,
    tenant_slug VARCHAR(255) NOT NULL,
    message TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    reviewed_by UUID REFERENCES identities(id) ON DELETE SET NULL,
    reviewed_at TIMESTAMPTZ,
    review_note TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE SET NULL
);

CREATE INDEX idx_tenant_creation_requests_applicant ON tenant_creation_requests(applicant_id);

CREATE INDEX idx_tenant_creation_requests_status ON tenant_creation_requests(status);

CREATE INDEX idx_tenant_creation_requests_created_at ON tenant_creation_requests(created_at DESC);
