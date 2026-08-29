-- Migration: 028_add_admin_unpublished
-- Description: Track whether a skill was unpublished by admin to prevent author/org from re-publishing

ALTER TABLE skills ADD COLUMN IF NOT EXISTS admin_unpublished BOOLEAN NOT NULL DEFAULT FALSE;
