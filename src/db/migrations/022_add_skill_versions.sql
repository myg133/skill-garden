-- 022_add_skill_versions.sql
-- Skill version history tracking with Git integration

CREATE TABLE IF NOT EXISTS skill_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_name VARCHAR(200) NOT NULL,
    version VARCHAR(50) NOT NULL,
    git_commit_hash VARCHAR(64),
    git_tag VARCHAR(100),
    changelog TEXT,
    file_count INTEGER NOT NULL DEFAULT 0,
    total_size_bytes BIGINT NOT NULL DEFAULT 0,
    uploaded_by UUID REFERENCES identities(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE(skill_name, version)
);

CREATE INDEX idx_skill_versions_name ON skill_versions(skill_name);
CREATE INDEX idx_skill_versions_created ON skill_versions(created_at DESC);

-- Add git repo tracking columns to skills
ALTER TABLE skills ADD COLUMN IF NOT EXISTS git_repo_name VARCHAR(200);
ALTER TABLE skills ADD COLUMN IF NOT EXISTS git_branch VARCHAR(100) DEFAULT 'main';

COMMENT ON TABLE skill_versions IS 'Tracks each skill version upload with corresponding Git commit references.';
