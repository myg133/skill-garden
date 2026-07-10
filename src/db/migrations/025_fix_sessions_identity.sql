-- Migration: 025_fix_sessions_identity
-- Description: Replace sessions.agent_id (VARCHAR FK→agents) with identity_id (UUID FK→identities)
-- Sessions now belong to identities (API key callers) instead of registered agents.

-- 1. Drop old FK and index
ALTER TABLE sessions DROP CONSTRAINT IF EXISTS fk_sessions_agent;
DROP INDEX IF EXISTS idx_sessions_agent;

-- 2. Drop old agent_id column
ALTER TABLE sessions DROP COLUMN IF EXISTS agent_id;

-- 3. Add identity_id column (UUID, references identities)
ALTER TABLE sessions ADD COLUMN IF NOT EXISTS identity_id UUID;

-- 4. Create new index
CREATE INDEX IF NOT EXISTS idx_sessions_identity ON sessions(identity_id);

-- 5. Add FK to identities table
ALTER TABLE sessions ADD CONSTRAINT fk_sessions_identity
    FOREIGN KEY (identity_id) REFERENCES identities(id) ON DELETE CASCADE;
