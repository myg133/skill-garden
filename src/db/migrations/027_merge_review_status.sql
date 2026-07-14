-- Merge review_status into status column
-- review_status was redundant with status; now status is the single source of truth
-- After this migration, status can be: draft, pending_review, approved, rejected, published

-- 1. Update any lingering "in_review" status to "pending_review"
UPDATE skills SET status = 'pending_review' WHERE status = 'in_review';

-- 2. Set default status for new rows
ALTER TABLE skills ALTER COLUMN status SET DEFAULT 'draft';

-- 3. Drop review_status column and its index
DROP INDEX IF EXISTS idx_skills_review_status;
ALTER TABLE skills DROP COLUMN IF EXISTS review_status;
