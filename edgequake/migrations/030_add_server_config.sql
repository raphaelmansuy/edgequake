-- Migration 030: Add server_config table for runtime admin settings
-- SPEC-0001: Tenant Workspace Limits (Issue #133)
--
-- WHY: Provides runtime-configurable server-wide defaults without redeployment.
-- The resolution order for new tenant default_max_workspaces:
--   1. server_config table → key "default_max_workspaces"
--   2. EDGEQUAKE_DEFAULT_MAX_WORKSPACES env var
--   3. TenantPlan::default_max_workspaces() (compile-time fallback)

CREATE TABLE IF NOT EXISTS server_config (
    key        TEXT PRIMARY KEY,
    value      JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default config (can be overridden at runtime)
INSERT INTO server_config (key, value, updated_at)
VALUES ('default_max_workspaces', '100'::jsonb, NOW())
ON CONFLICT (key) DO NOTHING;
