-- Migration 039: 清理未使用的 organization_identities 表
-- 该表在 014 中创建，但 Rust 代码中从未使用（无 model/repository/handler），属于孤儿表

DROP TABLE IF EXISTS organization_identities CASCADE;
