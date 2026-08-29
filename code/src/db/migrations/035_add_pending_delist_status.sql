-- Migration: 034_add_pending_delist_status
-- Description: Add 'pending_delist' to marketplace_status CHECK constraint
--              for the skill author request-delist workflow.

-- Step 1: Drop existing constraint
ALTER TABLE skills DROP CONSTRAINT IF EXISTS chk_marketplace_status;

-- Step 2: Re-add with 'pending_delist' included
ALTER TABLE skills ADD CONSTRAINT chk_marketplace_status CHECK (
    marketplace_status IS NULL
    OR marketplace_status IN ('pending_review', 'listed', 'rejected', 'delisted', 'unlisted', 'pending_delist')
);
