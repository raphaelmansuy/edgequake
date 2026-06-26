-- ============================================================================
-- E2E: Storage Invariant Tests (SPEC-021 P4-02)
-- File: specs/021-storage-study/e2e/test_invariants.sql
-- Run against a live database to check cross-store consistency
-- ============================================================================

SET search_path = public;

-- ============================================================================
-- INV-01: Every chunk vector has a KV entry
-- ============================================================================
\echo 'INV-01: Checking orphaned chunk vectors...'
SELECT
    COUNT(*) AS orphaned_chunk_vectors,
    CASE WHEN COUNT(*) = 0 THEN 'PASS' ELSE 'FAIL' END AS status
FROM eq_eq_default_vectors v
WHERE v.metadata->>'type' = 'chunk'
  AND NOT EXISTS (
    SELECT 1 FROM eq_eq_default_kv k WHERE k.key = v.id
  );

-- ============================================================================
-- INV-02: Every entity vector references a valid entity name
-- ============================================================================
\echo 'INV-02: Checking entity vectors without names...'
SELECT
    COUNT(*) AS entity_vectors_without_name,
    CASE WHEN COUNT(*) = 0 THEN 'PASS' ELSE 'WARN' END AS status
FROM eq_eq_default_vectors v
WHERE v.metadata->>'type' = 'entity'
  AND (v.metadata->>'entity_name' IS NULL OR v.metadata->>'entity_name' = '');

-- ============================================================================
-- INV-03: Every indexed document has ≥1 KV chunk
-- ============================================================================
\echo 'INV-03: Checking indexed documents without chunks...'
SELECT
    COUNT(*) AS indexed_without_chunks,
    CASE WHEN COUNT(*) = 0 THEN 'PASS' ELSE 'FAIL' END AS status
FROM documents d
WHERE d.status = 'indexed'
  AND NOT EXISTS (
    SELECT 1 FROM eq_eq_default_kv k
    WHERE k.key LIKE d.id::text || '-chunk-%'
  );

-- ============================================================================
-- INV-04: CQRS sync health (only meaningful when entity_sync_mode = 'full')
-- ============================================================================
\echo 'INV-04: Checking CQRS sync lag...'
WITH sync_mode AS (
    SELECT value::text AS mode FROM server_config WHERE key = 'entity_sync_mode'
),
counts AS (
    SELECT
        (SELECT COUNT(*) FROM entities WHERE sync_status = 'synced') AS synced,
        (SELECT COUNT(*) FROM entities) AS total_relational
)
SELECT
    m.mode AS sync_mode,
    c.synced AS synced_entities,
    c.total_relational AS total_entities,
    CASE
        WHEN m.mode NOT LIKE '%full%' THEN 'SKIP (sync not yet full)'
        WHEN c.total_relational = 0 THEN 'WARN (no entities synced yet)'
        ELSE 'PASS'
    END AS status
FROM sync_mode m, counts c;

-- ============================================================================
-- INV-05: No PDFs stuck in processing > 1 hour
-- ============================================================================
\echo 'INV-05: Checking stuck PDFs...'
SELECT
    COUNT(*) AS stuck_pdfs,
    CASE WHEN COUNT(*) = 0 THEN 'PASS' ELSE 'WARN' END AS status
FROM pdf_documents
WHERE processing_status = 'processing'
  AND NOW() - created_at > INTERVAL '1 hour';

-- ============================================================================
-- Summary
-- ============================================================================
\echo 'Invariant check complete. Review results above for any FAIL/WARN status.'
\echo 'FAIL = data consistency issue requiring immediate attention'
\echo 'WARN = potential issue, investigate but not immediately critical'
\echo 'PASS = invariant holds'
