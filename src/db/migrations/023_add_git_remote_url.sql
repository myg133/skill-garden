-- 023: Add git_remote_url columns for GitLab integration
ALTER TABLE skills ADD COLUMN IF NOT EXISTS git_remote_url TEXT;
ALTER TABLE skill_versions ADD COLUMN IF NOT EXISTS git_remote_url TEXT;
