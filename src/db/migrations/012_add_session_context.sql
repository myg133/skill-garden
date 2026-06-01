-- Migration: 012_add_session_context
-- Description: Add session context data for storing skill states, tool execution history, and workflow data

-- Enable UUID generation
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Drop existing tables if they exist (handles partial migration failures)
DROP TABLE IF EXISTS skill_dependencies CASCADE;
DROP TABLE IF EXISTS session_tool_executions CASCADE;
DROP TABLE IF EXISTS session_skills CASCADE;
DROP TABLE IF EXISTS session_context CASCADE;

-- Session context table for storing Agent's working context during a session
CREATE TABLE session_context (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    context_key VARCHAR(255) NOT NULL,
    context_value JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, context_key)
);

CREATE INDEX idx_session_context_session_id ON session_context(session_id);

-- Session skill state table for tracking which skills are active/loaded in a session
CREATE TABLE session_skills (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    skill_id VARCHAR(255) NOT NULL,
    skill_state JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(50) NOT NULL DEFAULT 'loaded',
    loaded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(session_id, skill_id)
);

CREATE INDEX idx_session_skills_session_id ON session_skills(session_id);
CREATE INDEX idx_session_skills_skill_id ON session_skills(skill_id);

-- Session tool execution history
CREATE TABLE session_tool_executions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    tool_id VARCHAR(255) NOT NULL,
    tool_type VARCHAR(50) NOT NULL,
    parameters JSONB NOT NULL DEFAULT '{}',
    result JSONB,
    success BOOLEAN NOT NULL DEFAULT false,
    execution_time_ms INTEGER,
    error_message TEXT,
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_session_tool_executions_session_id ON session_tool_executions(session_id);
CREATE INDEX idx_session_tool_executions_tool_id ON session_tool_executions(tool_id);
CREATE INDEX idx_session_tool_executions_executed_at ON session_tool_executions(executed_at);

-- Skill dependencies table for tracking and resolving skill dependencies
CREATE TABLE skill_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    skill_id VARCHAR(255) NOT NULL,
    dependency_skill_id VARCHAR(255) NOT NULL,
    version_constraint VARCHAR(50) NOT NULL DEFAULT '*',
    is_optional BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(skill_id, dependency_skill_id)
);

CREATE INDEX idx_skill_dependencies_skill_id ON skill_dependencies(skill_id);
CREATE INDEX idx_skill_dependencies_dependency ON skill_dependencies(dependency_skill_id);
