-- Initial database schema for AionHive
-- Migration: 001_initial_schema

-- Enable extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Agents table for JWT authentication
CREATE TABLE agents (
    agent_id VARCHAR(255) PRIMARY KEY,
    agent_secret_hash VARCHAR(255) NOT NULL,
    agent_name VARCHAR(255),
    roles TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Skills table
CREATE TABLE skills (
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    version VARCHAR(50) NOT NULL,
    author_agent_id VARCHAR(255) NOT NULL REFERENCES agents(agent_id),
    compatibility VARCHAR(100) NOT NULL DEFAULT '>=1.0.0',
    content TEXT NOT NULL DEFAULT '',
    install_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(name, version)
);

-- Skill tags (many-to-many)
CREATE TABLE skill_tags (
    skill_id VARCHAR(255) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    tag VARCHAR(100) NOT NULL,
    PRIMARY KEY (skill_id, tag)
);

-- Skill dependencies (many-to-many)
CREATE TABLE skill_dependencies (
    skill_id VARCHAR(255) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    dependency_id VARCHAR(255) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    PRIMARY KEY (skill_id, dependency_id)
);

-- Evaluations table
CREATE TABLE evaluations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    skill_id VARCHAR(255) NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
    agent_id VARCHAR(255) NOT NULL REFERENCES agents(agent_id),
    success BOOLEAN NOT NULL,
    duration_ms BIGINT NOT NULL,
    error_type VARCHAR(50),
    tags TEXT[] DEFAULT '{}',
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_skills_name ON skills(name);
CREATE INDEX idx_skills_author ON skills(author_agent_id);
CREATE INDEX idx_skills_created ON skills(created_at DESC);
CREATE INDEX idx_evaluations_skill ON evaluations(skill_id);
CREATE INDEX idx_evaluations_agent ON evaluations(agent_id);
CREATE INDEX idx_evaluations_timestamp ON evaluations(timestamp DESC);
CREATE INDEX idx_skill_tags_tag ON skill_tags(tag);

-- Audit log table
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    agent_id VARCHAR(255),
    action VARCHAR(100) NOT NULL,
    resource_type VARCHAR(50) NOT NULL,
    resource_id VARCHAR(255),
    details JSONB DEFAULT '{}',
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_timestamp ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_agent ON audit_logs(agent_id);
