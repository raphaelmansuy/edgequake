-- SPEC-149: event-scoped projection membership and separate provider receipts.
-- Additive. Historical migrations 150-154 stay unchanged.

SET search_path = public;

CREATE TABLE IF NOT EXISTS projection_event_items (
    event_id UUID NOT NULL REFERENCES projection_events(event_id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('graph', 'vector')),
    ordinal INT NOT NULL CHECK (ordinal >= 0),
    item_kind TEXT NOT NULL CHECK (char_length(item_kind) > 0),
    record_id UUID NOT NULL,
    record_revision BIGINT NOT NULL CHECK (record_revision > 0),
    digest BYTEA NOT NULL CHECK (octet_length(digest) = 32),
    logical_key TEXT NOT NULL CHECK (char_length(logical_key) > 0),
    PRIMARY KEY (event_id, role, ordinal),
    UNIQUE (event_id, role, item_kind, record_id, record_revision)
);

CREATE INDEX IF NOT EXISTS idx_projection_event_items_record
    ON projection_event_items (event_id, role, record_id);

CREATE TABLE IF NOT EXISTS projection_event_role_proofs (
    event_id UUID NOT NULL REFERENCES projection_events(event_id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('graph', 'vector')),
    expected_digest BYTEA NOT NULL CHECK (octet_length(expected_digest) = 32),
    PRIMARY KEY (event_id, role)
);

ALTER TABLE projection_deliveries
    ADD COLUMN IF NOT EXISTS provider_receipt TEXT;
