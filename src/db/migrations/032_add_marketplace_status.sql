-- Migration: 032_add_marketplace_status
-- Description: Add marketplace_status and pre_marketplace_visibility columns
--              to skills table for dual-track publish/marketplace model.
--              Phase 0 of permission-audit refactoring.

-- Step 1: Add marketplace_status column
-- Values: null (not submitted), pending_review, listed, rejected, delisted, unlisted
ALTER TABLE skills ADD COLUMN IF NOT EXISTS marketplace_status VARCHAR(50) DEFAULT NULL;
ALTER TABLE skills ADD CONSTRAINT chk_marketplace_status CHECK (
    marketplace_status IS NULL
    OR marketplace_status IN ('pending_review', 'listed', 'rejected', 'delisted', 'unlisted')
);

-- Step 2: Add pre_marketplace_visibility column
-- Saves the original visibility before submission, restored on delist/unlist
ALTER TABLE skills ADD COLUMN IF NOT EXISTS pre_marketplace_visibility VARCHAR(50) DEFAULT NULL;
ALTER TABLE skills ADD CONSTRAINT chk_pre_marketplace_visibility CHECK (
    pre_marketplace_visibility IS NULL
    OR pre_marketplace_visibility IN ('private', 'org_visible', 'marketplace', 'shared')
);

-- Step 3: Add index on marketplace_status for query performance
CREATE INDEX IF NOT EXISTS idx_skills_marketplace_status ON skills(marketplace_status);
-- Composite index for marketplace queries
CREATE INDEX IF NOT EXISTS idx_skills_status_marketplace ON skills(status, marketplace_status);

-- Step 4: Data migration — populate marketplace_status for existing marketplace skills
-- Skills that are published + visibility=marketplace → marketplace_status = 'listed'
UPDATE skills
SET marketplace_status = 'listed',
    pre_marketplace_visibility = 'marketplace'
WHERE status = 'published'
  AND visibility = 'marketplace'
  AND marketplace_status IS NULL;

-- Step 5: Mark admin_unpublished column as deprecated (kept for backward compat)
-- We'll remove it in a later migration after all code paths are updated
COMMENT ON COLUMN skills.admin_unpublished IS 'DEPRECATED: use marketplace_status instead. Remove in Phase 4.';
