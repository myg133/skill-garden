-- Migration: 036_add_draft_content
-- Description: Add draft_content column + extend marketplace_status/status CHECK constraints

ALTER TABLE skills ADD COLUMN IF NOT EXISTS draft_content JSONB DEFAULT NULL;

ALTER TABLE skills DROP CONSTRAINT IF EXISTS chk_marketplace_status;
ALTER TABLE skills ADD CONSTRAINT chk_marketplace_status CHECK (
    marketplace_status IS NULL
    OR marketplace_status IN (
        'pending_review', 'listed', 'rejected', 'delisted', 'unlisted',
        'pending_delist', 'pending_update'
    )
);

ALTER TABLE skills DROP CONSTRAINT IF EXISTS skills_status_check;
ALTER TABLE skills ADD CONSTRAINT chk_skills_status CHECK (
    status IN ('draft', 'pending_review', 'approved', 'rejected', 'published', 'archived')
);
