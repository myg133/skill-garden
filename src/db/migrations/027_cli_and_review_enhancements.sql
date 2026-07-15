-- Migration: 027_cli_and_review_enhancements
-- Description: Merge review_status into status, add CLI download fields, add config_data to download_tokens
-- Part 1: Merge review_status into status column
-- Part 2: Add resource_type + target columns for CLI binary downloads
-- Part 3: Add config_data column to store CLI config.toml content

-- ==========================================================
-- Part 1: Merge review_status into status
-- ==========================================================
-- review_status was redundant with status - status is now the single source of truth
-- After this migration, status can be: draft, pending_review, approved, rejected, published

-- 1.1 Expand the CHECK constraint to include 'approved'
ALTER TABLE skills DROP CONSTRAINT IF EXISTS skills_status_check;
ALTER TABLE skills ADD CONSTRAINT skills_status_check CHECK (status IN ('draft', 'pending_review', 'published', 'rejected', 'approved'));

-- 1.2 Update any lingering "in_review" status to "pending_review"
UPDATE skills SET status = 'pending_review' WHERE status = 'in_review';

-- 1.3 Set default status for new rows
ALTER TABLE skills ALTER COLUMN status SET DEFAULT 'draft';

-- 1.4 Drop review_status column and its index
DROP INDEX IF EXISTS idx_skills_review_status;
ALTER TABLE skills DROP COLUMN IF EXISTS review_status;

-- ==========================================================
-- Part 2: Add resource_type + target columns for CLI downloads
-- ==========================================================
-- resource_type: 'skill' for skill downloads, 'cli' for CLI binary downloads
ALTER TABLE download_tokens ADD COLUMN IF NOT EXISTS resource_type VARCHAR(10) NOT NULL DEFAULT 'skill';
-- target: CLI download target in {os}-{arch} format, e.g. linux-x86_64 (NULL for skill downloads)
ALTER TABLE download_tokens ADD COLUMN IF NOT EXISTS target VARCHAR(100);

-- ==========================================================
-- Part 3: Add config_data column for CLI config embedding
-- ==========================================================
-- Stores the config.toml content (with real API key) at token creation time.
-- It is embedded into the tar.gz when the agent downloads via cli.setup.
-- Only populated when resource_type = 'cli'.

ALTER TABLE download_tokens ADD COLUMN IF NOT EXISTS config_data TEXT;
