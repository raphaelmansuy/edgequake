-- Migration 150: SPEC-146 document ABAC schema
-- LAW-146-8,14,19,22,25 — security columns, tagged ACL, policy_generation, break-glass.

-- Documents security columns (defaults preserve pre-146 workspace-only behavior)
ALTER TABLE documents
  ADD COLUMN IF NOT EXISTS classification TEXT NOT NULL DEFAULT 'internal',
  ADD COLUMN IF NOT EXISTS share_mode TEXT NOT NULL DEFAULT 'workspace',
  ADD COLUMN IF NOT EXISTS export_control BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS pii BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS project_id TEXT,
  ADD COLUMN IF NOT EXISTS owner_org TEXT,
  ADD COLUMN IF NOT EXISTS owner_principal_kind TEXT NOT NULL DEFAULT 'user',
  ADD COLUMN IF NOT EXISTS owner_principal_id TEXT,
  ADD COLUMN IF NOT EXISTS retention_until TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS policy_etag TEXT,
  ADD COLUMN IF NOT EXISTS security_status TEXT NOT NULL DEFAULT 'ok',
  ADD COLUMN IF NOT EXISTS security_attrs JSONB NOT NULL DEFAULT '{}';

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'documents_share_mode_check'
  ) THEN
    ALTER TABLE documents
      ADD CONSTRAINT documents_share_mode_check
      CHECK (share_mode IN ('workspace', 'acl', 'classified', 'owner_only'));
  END IF;
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint WHERE conname = 'documents_security_status_check'
  ) THEN
    ALTER TABLE documents
      ADD CONSTRAINT documents_security_status_check
      CHECK (security_status IN ('ok', 'quarantined'));
  END IF;
END $$;

-- LAW-146-14: legacy backfill is the DEFAULT ('workspace' / 'ok') on ADD COLUMN.

CREATE INDEX IF NOT EXISTS idx_documents_ws_share
  ON documents (workspace_id, share_mode, security_status);
CREATE INDEX IF NOT EXISTS idx_documents_ws_class
  ON documents (workspace_id, classification);
CREATE INDEX IF NOT EXISTS idx_documents_owner
  ON documents (owner_principal_kind, owner_principal_id);

-- Tagged ACL (LAW-146-19)
CREATE TABLE IF NOT EXISTS document_acl (
  document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
  principal_kind TEXT NOT NULL,
  principal_id TEXT NOT NULL,
  permission TEXT NOT NULL,
  granted_by_kind TEXT,
  granted_by_id TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (document_id, principal_kind, principal_id, permission)
);
CREATE INDEX IF NOT EXISTS idx_document_acl_principal
  ON document_acl (principal_kind, principal_id);

-- Attribute catalog
CREATE TABLE IF NOT EXISTS attribute_definitions (
  attr_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  scope TEXT NOT NULL,
  name TEXT NOT NULL,
  value_type TEXT NOT NULL,
  enum_values JSONB,
  required_for_share_modes TEXT[] DEFAULT '{}',
  UNIQUE (workspace_id, scope, name)
);

CREATE TABLE IF NOT EXISTS principal_attributes (
  workspace_id UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  principal_kind TEXT NOT NULL,
  principal_id TEXT NOT NULL,
  name TEXT NOT NULL,
  value JSONB NOT NULL,
  source TEXT NOT NULL DEFAULT 'manual',
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (workspace_id, principal_kind, principal_id, name)
);

-- Workspace roles
CREATE TABLE IF NOT EXISTS workspace_roles (
  role_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
  permissions TEXT[] NOT NULL DEFAULT '{}',
  UNIQUE (workspace_id, name)
);

CREATE TABLE IF NOT EXISTS workspace_role_bindings (
  workspace_id UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  principal_kind TEXT NOT NULL,
  principal_id TEXT NOT NULL,
  role_id UUID NOT NULL REFERENCES workspace_roles(role_id) ON DELETE CASCADE,
  PRIMARY KEY (workspace_id, principal_kind, principal_id, role_id)
);

-- Policy store (PAP) — Cedar text for classified only
CREATE TABLE IF NOT EXISTS policies (
  policy_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  active_version BIGINT NOT NULL DEFAULT 0,
  UNIQUE (workspace_id, name)
);

CREATE TABLE IF NOT EXISTS policy_versions (
  policy_id UUID NOT NULL REFERENCES policies(policy_id) ON DELETE CASCADE,
  version BIGINT NOT NULL,
  cedar_text TEXT NOT NULL,
  cedar_hash CHAR(64) NOT NULL,
  schema_hash CHAR(64) NOT NULL,
  created_by UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (policy_id, version)
);

-- LAW-146-22: monotonic policy_generation per workspace
CREATE TABLE IF NOT EXISTS workspace_authz_state (
  workspace_id UUID PRIMARY KEY REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  policy_generation BIGINT NOT NULL DEFAULT 1,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- LAW-146-25: break-glass sessions
CREATE TABLE IF NOT EXISTS break_glass_sessions (
  session_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
  principal_kind TEXT NOT NULL,
  principal_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  expires_at TIMESTAMPTZ NOT NULL,
  scope_doc_ids UUID[],
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  revoked_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_break_glass_active
  ON break_glass_sessions (workspace_id, expires_at)
  WHERE revoked_at IS NULL;

COMMENT ON TABLE document_acl IS 'SPEC-146 tagged principal ACL rows';
COMMENT ON TABLE workspace_authz_state IS 'SPEC-146 monotonic policy_generation for cache/audit';
COMMENT ON COLUMN documents.policy_etag IS 'Content hash of document security labels (not policy_generation)';
