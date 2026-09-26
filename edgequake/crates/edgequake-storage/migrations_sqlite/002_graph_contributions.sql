-- Additive: graph contribution lineage for SQLite authority (SPEC-149 DRY/LSP).
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
