-- Add status column to skills table
ALTER TABLE skills ADD COLUMN status VARCHAR(20) NOT NULL DEFAULT 'pending_review' CHECK (status IN ('draft', 'pending_review', 'published', 'rejected'));

-- Index for filtering by status
CREATE INDEX idx_skills_status ON skills(status);

-- Index for listing pending review skills
CREATE INDEX idx_skills_status_created ON skills(status, created_at DESC);