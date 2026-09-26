-- ============================================================================
-- Migration 150: provider-access durable authority and projection ledger
-- SPEC-149 PROVIDER-ACCESS J08
-- ============================================================================
-- Additive and self-contained: no document/workspace foreign keys are used so
-- this ledger can be installed before later scoped-identity reconciliation.

SET search_path = public;

CREATE TABLE IF NOT EXISTS mutation_requests (
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    operation TEXT NOT NULL CHECK (char_length(operation) > 0),
    idempotency_key TEXT NOT NULL,
    digest BYTEA NOT NULL CHECK (octet_length(digest) = 32),
    receipt BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, workspace_id, operation, idempotency_key),
    CHECK (char_length(idempotency_key) > 0 AND char_length(idempotency_key) <= 256)
);

CREATE TABLE IF NOT EXISTS object_revisions (
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    kind TEXT NOT NULL CHECK (char_length(kind) > 0),
    logical_id UUID NOT NULL,
    revision BIGINT NOT NULL CHECK (revision > 0),
    state TEXT NOT NULL CHECK (state IN ('staged', 'active', 'tombstoned')),
    physical_id UUID NOT NULL,
    digest BYTEA NOT NULL CHECK (octet_length(digest) = 32),
    payload_ref TEXT,
    -- Exact prepared bytes make replay/digest verification independent of a
    -- projection provider. Large-object migration can move these behind
    -- payload_ref later without weakening the authority contract.
    payload BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, workspace_id, kind, logical_id, revision),
    UNIQUE (physical_id)
);

CREATE TABLE IF NOT EXISTS graph_contributions (
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    fact_id UUID NOT NULL,
    fact_revision BIGINT NOT NULL CHECK (fact_revision > 0),
    contribution_id UUID NOT NULL,
    source_document_id UUID NOT NULL,
    source_generation BIGINT NOT NULL CHECK (source_generation > 0),
    source_chunk_id UUID,
    payload_digest BYTEA NOT NULL CHECK (octet_length(payload_digest) = 32),
    payload JSONB NOT NULL,
    PRIMARY KEY (contribution_id),
    UNIQUE (
        tenant_id,
        workspace_id,
        source_document_id,
        source_generation,
        contribution_id
    )
);

CREATE INDEX IF NOT EXISTS idx_graph_contrib_by_doc
    ON graph_contributions (
        tenant_id,
        workspace_id,
        source_document_id,
        source_generation
    );

CREATE INDEX IF NOT EXISTS idx_graph_contrib_by_fact
    ON graph_contributions (
        tenant_id,
        workspace_id,
        fact_id,
        fact_revision
    );

CREATE TABLE IF NOT EXISTS embedding_manifests (
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    subject_id UUID NOT NULL,
    family TEXT NOT NULL CHECK (char_length(family) > 0),
    model_revision TEXT NOT NULL CHECK (char_length(model_revision) > 0),
    content_revision BIGINT NOT NULL CHECK (content_revision > 0),
    physical_id UUID NOT NULL,
    dimension INT NOT NULL CHECK (dimension >= 0),
    metric TEXT NOT NULL CHECK (char_length(metric) > 0),
    digest BYTEA NOT NULL CHECK (octet_length(digest) = 32),
    payload_ref TEXT,
    payload BYTEA,
    PRIMARY KEY (
        tenant_id,
        workspace_id,
        subject_id,
        family,
        model_revision,
        content_revision
    ),
    UNIQUE (physical_id)
);

CREATE TABLE IF NOT EXISTS ingest_batches (
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    document_id UUID NOT NULL,
    generation BIGINT NOT NULL CHECK (generation > 0),
    batch_ordinal INT NOT NULL CHECK (batch_ordinal >= 0),
    digest BYTEA NOT NULL CHECK (octet_length(digest) = 32),
    expected_count INT NOT NULL CHECK (expected_count >= 0),
    state TEXT NOT NULL CHECK (state IN ('staged', 'complete', 'published', 'failed')),
    PRIMARY KEY (
        tenant_id,
        workspace_id,
        document_id,
        generation,
        batch_ordinal
    )
);

CREATE TABLE IF NOT EXISTS data_bindings (
    binding_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    role TEXT NOT NULL CHECK (char_length(role) > 0),
    provider TEXT NOT NULL CHECK (char_length(provider) > 0),
    config_ref TEXT NOT NULL CHECK (char_length(config_ref) > 0),
    layout TEXT NOT NULL CHECK (char_length(layout) > 0),
    physical_index TEXT NOT NULL CHECK (char_length(physical_index) > 0),
    model_descriptor TEXT,
    generation BIGINT NOT NULL CHECK (generation > 0),
    state TEXT NOT NULL CHECK (state IN ('active', 'draining', 'retired')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_data_bindings_active
    ON data_bindings (
        tenant_id,
        workspace_id,
        role,
        COALESCE(model_descriptor, '')
    )
    WHERE state = 'active';

CREATE TABLE IF NOT EXISTS projection_events (
    event_id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    object_kind TEXT NOT NULL CHECK (char_length(object_kind) > 0),
    object_id UUID NOT NULL,
    object_revision BIGINT NOT NULL CHECK (object_revision > 0),
    schema_version INT NOT NULL CHECK (schema_version > 0),
    operation TEXT NOT NULL CHECK (char_length(operation) > 0),
    manifest_ref TEXT NOT NULL CHECK (char_length(manifest_ref) > 0),
    digest BYTEA NOT NULL CHECK (octet_length(digest) = 32),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (
        tenant_id,
        workspace_id,
        object_kind,
        object_id,
        object_revision,
        operation,
        schema_version
    )
);

CREATE TABLE IF NOT EXISTS projection_deliveries (
    event_id UUID NOT NULL REFERENCES projection_events(event_id) ON DELETE CASCADE,
    binding_id UUID NOT NULL REFERENCES data_bindings(binding_id),
    state TEXT NOT NULL CHECK (
        state IN ('pending', 'leased', 'retry', 'applied', 'quarantined')
    ),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    lease_until TIMESTAMPTZ,
    lease_owner UUID,
    epoch BIGINT NOT NULL DEFAULT 0 CHECK (epoch >= 0),
    attempts INT NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    receipt BYTEA,
    PRIMARY KEY (event_id, binding_id),
    CHECK (
        (state = 'leased' AND lease_until IS NOT NULL AND lease_owner IS NOT NULL)
        OR
        (state <> 'leased' AND lease_until IS NULL AND lease_owner IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_projection_deliveries_due
    ON projection_deliveries (next_attempt_at, event_id, binding_id)
    WHERE state IN ('pending', 'retry');

CREATE INDEX IF NOT EXISTS idx_projection_deliveries_lease
    ON projection_deliveries (lease_until, event_id, binding_id)
    WHERE state = 'leased';

CREATE TABLE IF NOT EXISTS projection_visibility (
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    object_kind TEXT NOT NULL CHECK (char_length(object_kind) > 0),
    object_id UUID NOT NULL,
    object_revision BIGINT NOT NULL CHECK (object_revision > 0),
    binding_id UUID NOT NULL REFERENCES data_bindings(binding_id),
    completion_receipt BYTEA NOT NULL,
    verified_generation BIGINT NOT NULL CHECK (verified_generation > 0),
    PRIMARY KEY (
        tenant_id,
        workspace_id,
        object_kind,
        object_id,
        object_revision,
        binding_id
    )
);

