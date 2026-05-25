-- Migration: 011_add_session_skill_fields
-- Description: Add capabilities and last_active_at to sessions, approved_at and approved_by to skills

-- Add columns to sessions table
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS capabilities JSONB DEFAULT '[]';
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS last_active_at TIMESTAMPTZ NOT NULL DEFAULT NOW();

-- Add columns to skills table
ALTER TABLE skills ADD COLUMN IF NOT EXISTS approved_at TIMESTAMPTZ;
ALTER TABLE skills ADD COLUMN IF NOT EXISTS approved_by VARCHAR(255);

-- Create index for session last_active_at
CREATE INDEX IF NOT EXISTS idx_sessions_last_active ON sessions(last_active_at);
