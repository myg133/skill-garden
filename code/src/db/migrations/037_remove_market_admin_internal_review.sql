-- Migration: 037_remove_marketplace_admin_internal_review
-- Description: Remove skill:approve_review and skill:reject_review from marketplace_admin role

DELETE FROM role_permissions
WHERE role_level = 'system'
  AND role_name = 'marketplace_admin'
  AND permission_code IN ('skill:approve_review', 'skill:reject_review');
