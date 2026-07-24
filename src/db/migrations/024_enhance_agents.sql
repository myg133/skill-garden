-- Migration: 024_enhance_agents
-- Description: Enhance agents table for API Key-based agent registration with identity linking
-- 运行在 023 之后

-- 1. 清理旧的、无 identity_id 关联的 agents（pre-RBAC 时代的遗留数据）
DELETE FROM agents WHERE identity_id IS NULL;

-- 2. 添加 agent token 相关字段
ALTER TABLE agents ADD COLUMN IF NOT EXISTS agent_token_hash VARCHAR(255);
ALTER TABLE agents ADD COLUMN IF NOT EXISTS agent_token_expires_at TIMESTAMPTZ;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS status VARCHAR(50) DEFAULT 'active';
ALTER TABLE agents ADD COLUMN IF NOT EXISTS last_used_at TIMESTAMPTZ;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS agent_description TEXT;
ALTER TABLE agents ADD COLUMN IF NOT EXISTS api_key_id UUID REFERENCES api_keys(id);

-- 3. 建立索引
CREATE INDEX IF NOT EXISTS idx_agents_identity_id ON agents(identity_id);
CREATE INDEX IF NOT EXISTS idx_agents_token_hash ON agents(agent_token_hash);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
CREATE INDEX IF NOT EXISTS idx_agents_api_key_id ON agents(api_key_id);

-- 4. 在 audit_log_entries 表中添加 agent_id 和 api_key_id 追踪
ALTER TABLE audit_log_entries ADD COLUMN IF NOT EXISTS api_key_id UUID REFERENCES api_keys(id);
ALTER TABLE audit_log_entries ADD COLUMN IF NOT EXISTS agent_id UUID;
