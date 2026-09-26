-- Event-scoped projection membership for the SQLite authority adapter.
CREATE TABLE IF NOT EXISTS projection_event_items (
    event_id TEXT NOT NULL REFERENCES projection_events(event_id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    ordinal INTEGER NOT NULL,
    item_kind TEXT NOT NULL,
    record_id TEXT NOT NULL,
    record_revision INTEGER NOT NULL,
    digest BLOB NOT NULL,
    logical_key TEXT NOT NULL,
    PRIMARY KEY (event_id, role, ordinal)
);

CREATE TABLE IF NOT EXISTS projection_event_role_proofs (
    event_id TEXT NOT NULL REFERENCES projection_events(event_id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    expected_digest BLOB NOT NULL,
    PRIMARY KEY (event_id, role)
);

CREATE TABLE IF NOT EXISTS projection_cleanup_intents (
    cleanup_manifest_id TEXT NOT NULL,
    binding_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    document_id TEXT NOT NULL,
    tombstone_revision INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',
    PRIMARY KEY (cleanup_manifest_id, binding_id)
);
