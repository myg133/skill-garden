-- Seed admin agent with admin role
-- Migration: 003_seed_admin_agent

-- Update existing admin-1 agent to have admin role (idempotent)
UPDATE agents
SET roles = ARRAY['admin'], updated_at = NOW()
WHERE agent_id = 'admin-1'
AND NOT ('admin' = ANY(roles));
