-- Migration: 010_add_admin_users
-- Create admin_users table for human administrators

CREATE TABLE admin_users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(255),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default admin user (password: admin123)
-- Hash generated with bcrypt cost 12: $2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/X4d5GIHN3GGaKTau
INSERT INTO admin_users (username, password_hash, display_name)
VALUES ('admin', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/X4d5GIHN3GGaKTau', 'Administrator')
ON CONFLICT (username) DO NOTHING;
