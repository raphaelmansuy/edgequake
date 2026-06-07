-- SPEC-016 / SPEC-007 M1: Complete the materialized-column backfill on vector tables.
-- @implements SPEC-007 R-T3-01, R-T3-04
--
-- WHY THIS MIGRATION EXISTS (gap closed):
-- Migration 028 added `document_id/tenant_id/workspace_id` and backfilled them,
-- but its PHASE 2 UPDATE was keyed on `WHERE document_id IS NULL` ONLY. Any row
-- that already had a non-NULL `document_id` but a NULL `tenant_id` or
-- `workspace_id` (e.g. single-tenant rows, or rows whose document_id was set by
-- a later code path) was therefore SKIPPED and still carries NULLs in the tenant
-- and workspace columns. That breaks column-only filters (QW6 workspace
-- counters, Tier-3 pre-filter) for those rows, silently forcing JSONB fallback.
--
-- This migration re-runs the backfill keyed independently on EACH column, so a
-- row is touched whenever ANY of the three columns is still NULL but recoverable
-- from JSONB metadata. It mirrors the spec M1 statement exactly:
--   COALESCE(col, metadata->>'...') WHERE document_id IS NULL OR tenant_id IS NULL OR workspace_id IS NULL
--
-- IDEMPOTENCY / SAFETY:
--   * COALESCE never overwrites an already-populated column.
--   * The WHERE clause excludes fully-populated rows, so re-running is a no-op.
--   * Rows whose metadata has none of the keys stay NULL (correct: the partial
--     indexes from 029 exclude them, and queries keep their JSONB fallback).
--   * Batched by `ctid` (LIMIT 10000) with a short sleep between batches to avoid
--     a long table-level lock on large corpora.
--   * Reversible: harmless. Re-running or rolling back changes nothing.

-- ============================================================
-- PHASE 1: Defensively ensure the materialized columns exist.
-- WHY (robustness): 037 logically runs after 028, but a vector table can match
-- the `eq_%_vectors` pattern yet lack the columns — e.g. a per-workspace table
-- created by application code AFTER 028 ran, or any table added between the two
-- migrations. `ADD COLUMN IF NOT EXISTS` is a no-op on tables that already have
-- the columns, so this keeps PHASE 2 from failing on a stray column-less table
-- while changing nothing for the common case.
-- ============================================================
DO $$
DECLARE
    tbl RECORD;
BEGIN
    FOR tbl IN
        SELECT tablename FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq_%_vectors'
    LOOP
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS document_id TEXT', tbl.tablename);
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS tenant_id TEXT', tbl.tablename);
        EXECUTE format('ALTER TABLE public.%I ADD COLUMN IF NOT EXISTS workspace_id TEXT', tbl.tablename);
    END LOOP;
END $$;

-- ============================================================
-- PHASE 2: Per-column COALESCE backfill (the gap 028 left open).
-- ============================================================
DO $$
DECLARE
    tbl RECORD;
    batch_size INT := 10000;
    updated INT;
BEGIN
    FOR tbl IN
        SELECT tablename FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq_%_vectors'
    LOOP
        LOOP
            EXECUTE format(
                'UPDATE public.%I SET
                    document_id  = COALESCE(document_id,  metadata->>''document_id'', metadata->>''source_document_id''),
                    tenant_id    = COALESCE(tenant_id,    metadata->>''tenant_id''),
                    workspace_id = COALESCE(workspace_id, metadata->>''workspace_id'')
                WHERE ctid IN (
                    SELECT ctid FROM public.%I
                    WHERE (document_id  IS NULL AND (metadata ? ''document_id'' OR metadata ? ''source_document_id''))
                       OR (tenant_id    IS NULL AND metadata ? ''tenant_id'')
                       OR (workspace_id IS NULL AND metadata ? ''workspace_id'')
                    LIMIT %s
                )',
                tbl.tablename, tbl.tablename, batch_size
            );
            GET DIAGNOSTICS updated = ROW_COUNT;
            EXIT WHEN updated < batch_size;
            PERFORM pg_sleep(0.05);
        END LOOP;
    END LOOP;
END $$;
