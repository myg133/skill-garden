-- Migration: 019_add_system_role_assignments
-- Description: Add system_role_assignments table for super_admin / marketplace_admin
--              system-level role assignments

CREATE TABLE system_role_assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    identity_id UUID NOT NULL REFERENCES identities(id) ON DELETE CASCADE,
    role_name VARCHAR(50) NOT NULL,
    assigned_by UUID REFERENCES identities(id),
    assigned_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(identity_id, role_name)
);

CREATE INDEX idx_system_role_assignments_identity ON system_role_assignments(identity_id);
CREATE INDEX idx_system_role_assignments_role ON system_role_assignments(role_name);