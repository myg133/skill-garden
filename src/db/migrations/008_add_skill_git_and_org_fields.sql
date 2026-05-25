-- Migration: 008_add_skill_git_and_org_fields
-- Description: Add git_url, visibility, skill_tools to skills and org_id, capabilities to agents

-- Add columns to skills table
ALTER TABLE skills ADD COLUMN git_url VARCHAR(500);
ALTER TABLE skills ADD COLUMN visibility VARCHAR(50) NOT NULL DEFAULT 'org_visible';
ALTER TABLE skills ADD COLUMN skill_tools JSONB DEFAULT '[]';

-- Add columns to agents table
ALTER TABLE agents ADD COLUMN org_id UUID;
ALTER TABLE agents ADD COLUMN capabilities JSONB DEFAULT '[]';

-- Create index for skills visibility
CREATE INDEX idx_skills_visibility ON skills(visibility);
