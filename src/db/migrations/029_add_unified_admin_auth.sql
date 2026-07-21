-- Migration: 029_add_unified_admin_auth
-- Description: Unified auth refactor (Phase 1-4)
--   Phase 1: Add system:admin:access + marketplace_admin route access
--   Phase 2: JWT sliming - assign super_admin to all is_system_admin users
--   Phase 3: marketplace_admin route-specific permissions
--   Phase 4: Cleanup - migrate is_system_admin → super_admin, deprecate legacy field

-- Add permission code for admin route access
INSERT INTO permissions (code, name, resource_type, action, scope) VALUES
    ('system:admin:access', 'Access Admin Routes', 'system', 'admin_access', 'none')
ON CONFLICT (code) DO NOTHING;

-- Grant to super_admin (all admin routes accessible)
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('system', 'super_admin', 'system:admin:access', 'none')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- Add permission code for marketplace-specific admin routes
INSERT INTO permissions (code, name, resource_type, action, scope) VALUES
    ('marketplace:manage', 'Manage Marketplace Skills', 'marketplace', 'manage', 'none')
ON CONFLICT (code) DO NOTHING;

-- Grant marketplace management permissions
INSERT INTO role_permissions (role_level, role_name, permission_code, scope_restriction) VALUES
    ('system', 'super_admin', 'marketplace:manage', 'none'),
    ('system', 'marketplace_admin', 'marketplace:manage', 'none')
ON CONFLICT (role_level, role_name, permission_code) DO NOTHING;

-- Phase 2 & 4: Assign super_admin system role to all legacy is_system_admin users
INSERT INTO system_role_assignments (identity_id, role_name, assigned_by)
SELECT id, 'super_admin', id
FROM identities
WHERE is_system_admin = true
  AND NOT EXISTS (
      SELECT 1 FROM system_role_assignments sra
      WHERE sra.identity_id = identities.id AND sra.role_name = 'super_admin'
  );
