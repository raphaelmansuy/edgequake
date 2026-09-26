-- SPEC-149 J14: standalone pgvector projection layout.
--
-- This table intentionally has no FK to chunks, documents, or another
-- relational authority. Canonical hydration is performed through injected
-- ports after scoped projection search.

SET search_path = public;

CREATE TABLE IF NOT EXISTS embedding_projections (
    tenant_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    family TEXT NOT NULL CHECK (char_length(family) > 0),
    subject_id UUID NOT NULL,
    model_revision TEXT NOT NULL CHECK (char_length(model_revision) > 0),
    content_revision BIGINT NOT NULL CHECK (content_revision > 0),
    physical_id UUID NOT NULL,
    embedding halfvec NOT NULL,
    dimensions INT NOT NULL CHECK (dimensions > 0),
    filter_payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    digest BYTEA NOT NULL CHECK (octet_length(digest) = 32),
    PRIMARY KEY (
        tenant_id,
        workspace_id,
        family,
        subject_id,
        model_revision,
        content_revision
    ),
    UNIQUE (physical_id)
);

CREATE INDEX IF NOT EXISTS idx_embedding_projections_scope_family
    ON embedding_projections (
        tenant_id,
        workspace_id,
        family,
        model_revision,
        content_revision
    );

CREATE INDEX IF NOT EXISTS idx_embedding_projections_filter_payload
    ON embedding_projections USING GIN (filter_payload jsonb_path_ops);
