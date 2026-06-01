-- Migration: 021_merge_admin_users_into_identities
-- Description: Merge admin_users table into identities, add is_system_admin flag
-- 运行在 020 之后

-- ============================================================
-- 1. 在 identities 表添加 is_system_admin 字段
-- ============================================================

ALTER TABLE identities ADD COLUMN IF NOT EXISTS is_system_admin BOOLEAN NOT NULL DEFAULT false;

-- ============================================================
-- 2. 迁移 admin_users 数据到 identities
-- ============================================================

INSERT INTO identities (id, identity_type, username, display_name, name, password_hash, status, is_system_admin, metadata, created_at, updated_at)
SELECT
    id,
    'system' AS identity_type,
    username,
    display_name,
    username AS name,
    password_hash,
    CASE WHEN is_active THEN 'active' ELSE 'inactive' END AS status,
    true AS is_system_admin,
    '{}'::jsonb AS metadata,
    created_at,
    updated_at
FROM admin_users
ON CONFLICT (id) DO NOTHING;

-- ============================================================
-- 3. 如果有 username 冲突，使用带后缀的 username 重试
-- ============================================================

INSERT INTO identities (id, identity_type, username, display_name, name, password_hash, status, is_system_admin, metadata, created_at, updated_at)
SELECT
    id,
    'system' AS identity_type,
    username || '_sys' AS username,
    display_name,
    username AS name,
    password_hash,
    CASE WHEN is_active THEN 'active' ELSE 'inactive' END AS status,
    true AS is_system_admin,
    '{}'::jsonb AS metadata,
    created_at,
    updated_at
FROM admin_users
WHERE id NOT IN (SELECT id FROM identities)
ON CONFLICT (username) DO NOTHING;

-- ============================================================
-- 4. 删除 admin_users 表
-- ============================================================

DROP TABLE IF EXISTS admin_users;