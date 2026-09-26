-- ============================================================================
-- Migration 159: SPEC-150 migration run telemetry + schema_compat
-- Version: 1.0.0 — 2026-09-26
--
-- PURPOSE:
--   Record per-migrate-run and per-version step timings so operators can
--   measure upgrade cost (WP-3 / WP-10). Also store `schema_compat` — the
--   minimum binary schema a newer ledger still allows older binaries to serve
--   (N-1 rolling window, WP-5).
--
-- SAFETY: idempotent CREATE IF NOT EXISTS; no data movement.
-- ============================================================================

SET search_path = public;

CREATE SCHEMA IF NOT EXISTS edgequake;

CREATE TABLE IF NOT EXISTS edgequake.migration_run (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    binary_version  TEXT NOT NULL,
    started_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    finished_at     TIMESTAMPTZ,
    outcome         TEXT NOT NULL DEFAULT 'running'
                    CHECK (outcome IN ('running', 'success', 'failed', 'tempfail', 'refused')),
    applied_count   INT NOT NULL DEFAULT 0,
    notes           TEXT
);

CREATE INDEX IF NOT EXISTS idx_edgequake_migration_run_started
    ON edgequake.migration_run (started_at DESC);

CREATE TABLE IF NOT EXISTS edgequake.migration_run_step (
    run_id          UUID NOT NULL REFERENCES edgequake.migration_run(id) ON DELETE CASCADE,
    version         BIGINT NOT NULL,
    phase           TEXT NOT NULL DEFAULT 'expand',
    duration_ms     BIGINT NOT NULL DEFAULT 0,
    sqlstate        TEXT,
    outcome         TEXT NOT NULL DEFAULT 'success',
    PRIMARY KEY (run_id, version)
);

CREATE TABLE IF NOT EXISTS edgequake.schema_compat (
    id                  INT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    min_binary_schema   BIGINT NOT NULL DEFAULT 0,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);

INSERT INTO edgequake.schema_compat (id, min_binary_schema)
VALUES (1, 0)
ON CONFLICT (id) DO NOTHING;
