-- Migration: 031_seed_admin_user
-- Description: Create an initial super_admin identity with a fixed UUID.
--              Default credentials: admin / admin123
--              CHANGE the password immediately after first login!

-- Fixed UUID for deterministic seeding
-- Step 1: Create the admin identity if no super_admin exists
INSERT INTO identities (
    id, identity_type, name, username, display_name, email,
    password_hash, is_system_admin, status, created_at, updated_at
)
SELECT
    '00000000-0000-0000-0000-000000000001', 'user', 'admin', 'admin', 'Super Admin',
    'admin@aionhive.local',
    '$2b$12$LJ3m4ys3GZfnYMz8kVsKaOlSiWhU2wZFPm./bCv4xJvK5pTM1XhKm',
    true, 'active', NOW(), NOW()
WHERE NOT EXISTS (
    SELECT 1 FROM identities WHERE is_system_admin = true
    UNION ALL
    SELECT 1 FROM system_role_assignments WHERE role_name = 'super_admin'
    LIMIT 1
);

-- Step 2: Assign super_admin system role to the seeded admin
INSERT INTO system_role_assignments (identity_id, role_name, assigned_by)
SELECT
    '00000000-0000-0000-0000-000000000001', 'super_admin', '00000000-0000-0000-0000-000000000001'
WHERE EXISTS (
    SELECT 1 FROM identities WHERE id = '00000000-0000-0000-0000-000000000001' AND is_system_admin = true
);
