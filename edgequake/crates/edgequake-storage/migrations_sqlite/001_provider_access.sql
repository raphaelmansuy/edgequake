PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS documents (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    revision INTEGER NOT NULL DEFAULT 0,
    deleted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (tenant_id, workspace_id, id)
);

CREATE TABLE IF NOT EXISTS mutation_requests (
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    digest BLOB NOT NULL,
    receipt BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (tenant_id, workspace_id, operation, idempotency_key)
);

CREATE TABLE IF NOT EXISTS object_revisions (
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    logical_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision > 0),
    state TEXT NOT NULL,
    physical_id TEXT NOT NULL,
    digest BLOB NOT NULL,
    payload BLOB NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, kind, logical_id, revision)
);

CREATE TABLE IF NOT EXISTS graph_contributions (
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    fact_id TEXT NOT NULL,
    fact_revision INTEGER NOT NULL,
    contribution_id TEXT PRIMARY KEY,
    source_document_id TEXT NOT NULL,
    source_generation INTEGER NOT NULL,
    payload_digest BLOB NOT NULL,
    payload BLOB NOT NULL,
    UNIQUE (
        tenant_id, workspace_id, source_document_id, source_generation, contribution_id
    )
);

CREATE TABLE IF NOT EXISTS ingest_batches (
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    batch_ordinal INTEGER NOT NULL,
    digest BLOB NOT NULL,
    expected_count INTEGER NOT NULL,
    state TEXT NOT NULL,
    PRIMARY KEY (tenant_id, workspace_id, document_id, generation, batch_ordinal)
);

CREATE TABLE IF NOT EXISTS data_bindings (
    binding_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    role TEXT NOT NULL,
    generation INTEGER NOT NULL DEFAULT 1,
    state TEXT NOT NULL DEFAULT 'active'
);

CREATE TABLE IF NOT EXISTS projection_events (
    event_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    object_kind TEXT NOT NULL,
    object_id TEXT NOT NULL,
    object_revision INTEGER NOT NULL,
    schema_version INTEGER NOT NULL,
    operation TEXT NOT NULL,
    manifest_ref TEXT NOT NULL,
    digest BLOB NOT NULL,
    UNIQUE (
        tenant_id, workspace_id, object_kind, object_id,
        object_revision, operation, schema_version
    )
);

CREATE TABLE IF NOT EXISTS projection_deliveries (
    event_id TEXT NOT NULL REFERENCES projection_events(event_id) ON DELETE CASCADE,
    binding_id TEXT NOT NULL REFERENCES data_bindings(binding_id) ON DELETE CASCADE,
    state TEXT NOT NULL DEFAULT 'pending',
    next_attempt_at INTEGER NOT NULL DEFAULT (unixepoch() * 1000),
    lease_until INTEGER,
    lease_owner TEXT,
    epoch INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    receipt BLOB,
    PRIMARY KEY (event_id, binding_id)
);

CREATE INDEX IF NOT EXISTS idx_projection_deliveries_due
    ON projection_deliveries (state, next_attempt_at, event_id, binding_id);

CREATE TABLE IF NOT EXISTS projection_visibility (
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    object_kind TEXT NOT NULL,
    object_id TEXT NOT NULL,
    object_revision INTEGER NOT NULL,
    binding_id TEXT NOT NULL,
    completion_receipt BLOB NOT NULL,
    verified_generation INTEGER NOT NULL,
    PRIMARY KEY (
        tenant_id, workspace_id, object_kind, object_id,
        object_revision, binding_id
    )
);

CREATE TABLE IF NOT EXISTS pipeline_checkpoints (
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (document_id, kind)
);

CREATE TABLE IF NOT EXISTS document_artifacts (
    document_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (document_id, kind)
);

CREATE TABLE IF NOT EXISTS users (
    user_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    username TEXT NOT NULL,
    email TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    failed_login_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_login_at TEXT,
    UNIQUE (tenant_id, username),
    UNIQUE (tenant_id, email)
);

CREATE TABLE IF NOT EXISTS workspaces (
    workspace_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (tenant_id, slug)
);

CREATE TABLE IF NOT EXISTS memberships (
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL REFERENCES workspaces(workspace_id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (tenant_id, workspace_id, user_id)
);

CREATE TABLE IF NOT EXISTS refresh_tokens (
    token_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    revoked_at TEXT
);

CREATE TABLE IF NOT EXISTS api_keys (
    key_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
    key_hash TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    name TEXT,
    scopes TEXT NOT NULL DEFAULT '[]',
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    last_used_at TEXT,
    expires_at TEXT
);
