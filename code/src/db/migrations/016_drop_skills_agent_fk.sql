-- Migration: 016_drop_skills_agent_fk
-- Remove FK constraint from skills.author_agent_id since admin users
-- (stored in admin_users table) can also create skills. The FK was
-- designed for an agent-only model and conflicts with the dual
-- identity system (agents + admin_users).

ALTER TABLE skills DROP CONSTRAINT IF EXISTS skills_author_agent_id_fkey;