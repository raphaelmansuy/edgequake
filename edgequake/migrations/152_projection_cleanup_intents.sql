-- SPEC-149 J12: durable targeted cleanup for tombstoned documents.

SET search_path = public;

CREATE TABLE IF NOT EXISTS projection_cleanup_intents (
    cleanup_manifest_id UUID NOT NULL,
    binding_id UUID NOT NULL REFERENCES data_bindings(binding_id),
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    document_id UUID NOT NULL,
    tombstone_revision BIGINT NOT NULL CHECK (tombstone_revision > 0),
    state TEXT NOT NULL DEFAULT 'pending'
        CHECK (state IN ('pending', 'leased', 'applied', 'quarantined')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (cleanup_manifest_id, binding_id)
);

CREATE INDEX IF NOT EXISTS idx_projection_cleanup_intents_document
    ON projection_cleanup_intents (
        tenant_id,
        workspace_id,
        document_id,
        tombstone_revision
    );
