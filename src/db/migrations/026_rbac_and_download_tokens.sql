-- Migration: 026_rbac_and_download_tokens
-- Description: RBAC support (nullable org for API keys, seed default role) + Download token audit trail
-- 运行在 025 之后

-- ============================================================
-- Part 1: Make api_keys.organization_id nullable (个人用户 API Key)
-- ============================================================

ALTER TABLE api_keys ALTER COLUMN organization_id DROP NOT NULL;

ALTER TABLE api_keys DROP CONSTRAINT IF EXISTS api_keys_organization_id_fkey;
ALTER TABLE api_keys ADD CONSTRAINT api_keys_organization_id_fkey
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE SET NULL;

COMMENT ON COLUMN api_keys.organization_id IS '组织 ID，个人用户创建的 API Key 可为空';

-- ============================================================
-- Part 2: Seed default 'skill_user' role
-- ============================================================

INSERT INTO roles (id, name, role_type, scope_level, permissions, description, created_at)
VALUES (
    gen_random_uuid(),
    'skill_user',
    'system',
    'global',
    '["skill:create", "skill:read", "skill:update", "skill:submit_review", "skill:install"]',
    '普通 Skill 用户，可创建和管理自己的 Skill，浏览技能市场',
    NOW()
)
ON CONFLICT (name, role_type) DO NOTHING;

-- ============================================================
-- Part 3: Download tokens table (技能下载凭证与审计)
-- ============================================================

CREATE TABLE IF NOT EXISTS download_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    token VARCHAR(64) NOT NULL UNIQUE,
    skill_name VARCHAR(255) NOT NULL,
    skill_version VARCHAR(100) NOT NULL,
    identity_id UUID NOT NULL REFERENCES identities(id),
    api_key_id UUID NOT NULL REFERENCES api_keys(id),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_download_tokens_token ON download_tokens(token);
CREATE INDEX IF NOT EXISTS idx_download_tokens_identity ON download_tokens(identity_id);
CREATE INDEX IF NOT EXISTS idx_download_tokens_api_key ON download_tokens(api_key_id);

COMMENT ON TABLE download_tokens IS '技能下载凭证表，记录每次 skills.install 生成的下载 token 及其使用者身份';
COMMENT ON COLUMN download_tokens.token IS '不透明随机 UUID 凭证，暴露在下载 URL 中';
COMMENT ON COLUMN download_tokens.used_at IS 'NULL=未下载，有值=已下载的时间';
