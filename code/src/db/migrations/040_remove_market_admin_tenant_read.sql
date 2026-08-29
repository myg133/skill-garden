-- Migration: 040_remove_market_admin_tenant_read
-- Description: Remove tenant:read permission from marketplace_admin role.
--              marketplace_admin does not need tenant management access;
--              this permission was causing the "Organizations" group title
--              to appear in the sidebar for marketplace_admin users.
--              Marketplace audit/review operations do not depend on tenant:read.

DELETE FROM role_permissions
WHERE role_level = 'system'
  AND role_name = 'marketplace_admin'
  AND permission_code = 'tenant:read';
