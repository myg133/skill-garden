-- Migration: 009_add_agent_id_column
-- Add missing 'id' UUID column to agents table

-- Add id column with default for new inserts
ALTER TABLE agents ADD COLUMN id UUID DEFAULT uuid_generate_v4();

-- Update existing rows with generated UUIDs
UPDATE agents SET id = uuid_generate_v4() WHERE id IS NULL;

-- Make it NOT NULL after backfilling
ALTER TABLE agents ALTER COLUMN id SET NOT NULL;

-- Add unique constraint since agent_id is already PK, this is just for internal reference
-- Note: We don't make it the primary key since agent_id is already the business key
