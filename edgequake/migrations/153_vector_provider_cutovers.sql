-- SPEC-149 J18: durable, resumable vector-provider cutover state.

SET search_path = public;

CREATE TABLE IF NOT EXISTS vector_provider_cutovers (
    cutover_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    old_binding_id UUID NOT NULL REFERENCES data_bindings(binding_id),
    new_binding_id UUID NOT NULL REFERENCES data_bindings(binding_id),
    backfill_cursor JSONB NOT NULL DEFAULT '{}'::jsonb,
    state TEXT NOT NULL CHECK (
        state IN ('backfilling', 'switched', 'rolled_back')
    ),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (old_binding_id <> new_binding_id),
    UNIQUE (old_binding_id, new_binding_id)
);

CREATE INDEX IF NOT EXISTS idx_vector_provider_cutovers_scope
    ON vector_provider_cutovers (tenant_id, workspace_id, state);
