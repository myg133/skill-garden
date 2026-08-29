-- Migration: 020_add_organization_slug_unique
-- Description: Add UNIQUE(tenant_id, slug) constraint on organizations
--              to ensure slug uniqueness within each tenant

ALTER TABLE organizations ADD CONSTRAINT organizations_tenant_slug_unique UNIQUE (tenant_id, slug);