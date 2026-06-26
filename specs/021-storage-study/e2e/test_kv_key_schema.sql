-- ============================================================================
-- E2E: KV Key Schema Contract Tests (SPEC-021 P1-01)
-- File: specs/021-storage-study/e2e/test_kv_key_schema.sql
-- Run with: cargo test -p edgequake-storage --lib kv_key_schema
-- Purpose: Verify KV key formats are stable and match expectations
-- ============================================================================

-- These SQL snippets are the production-database equivalents of the Rust unit
-- tests in edgequake-storage/src/kv_key_schema.rs.
-- Run against a live database to verify the KV store uses expected key formats.

-- ============================================================================
-- Test 1: Find all distinct key suffix patterns in the KV table
-- Expected: all suffixes should be -metadata, -chunk-{N}, -content, or -cache
-- ============================================================================
WITH key_patterns AS (
    SELECT
        CASE
            WHEN key LIKE '%-metadata'     THEN '-metadata'
            WHEN key ~ '-chunk-[0-9]+$'    THEN '-chunk-{n}'
            WHEN key LIKE '%-content'      THEN '-content'
            WHEN key LIKE '%-cache'        THEN '-cache'
            WHEN key LIKE '%-kwcache'      THEN '-kwcache'
            ELSE 'UNKNOWN: ' || right(key, 20)
        END AS pattern,
        COUNT(*) AS count
    FROM eq_eq_default_kv
    GROUP BY 1
)
SELECT pattern, count FROM key_patterns ORDER BY count DESC;
-- EXPECT: Only known patterns (no UNKNOWN rows)

-- ============================================================================
-- Test 2: Verify metadata keys have corresponding chunk keys
-- ============================================================================
WITH meta_docs AS (
    SELECT left(key, length(key) - 9) AS doc_id  -- strip '-metadata'
    FROM eq_eq_default_kv
    WHERE key LIKE '%-metadata'
),
chunks_per_doc AS (
    SELECT
        m.doc_id,
        COUNT(k.key) AS chunk_count
    FROM meta_docs m
    LEFT JOIN eq_eq_default_kv k
        ON k.key LIKE m.doc_id || '-chunk-%'
    GROUP BY m.doc_id
)
SELECT doc_id, chunk_count
FROM chunks_per_doc
WHERE chunk_count = 0
ORDER BY doc_id;
-- EXPECT: empty result (every document with metadata should have at least 1 chunk)
-- If non-empty: these documents were processed but chunk storage failed (INV-03)

-- ============================================================================
-- Test 3: Verify chunk keys are contiguous (no gaps in chunk_index)
-- ============================================================================
WITH chunk_keys AS (
    SELECT
        substring(key from '(.+)-chunk-[0-9]+$') AS doc_id,
        (regexp_match(key, '-chunk-([0-9]+)$'))[1]::int AS chunk_idx
    FROM eq_eq_default_kv
    WHERE key ~ '-chunk-[0-9]+$'
),
max_per_doc AS (
    SELECT doc_id, MAX(chunk_idx) AS max_idx, COUNT(*) AS actual_count
    FROM chunk_keys
    GROUP BY doc_id
)
SELECT doc_id, max_idx + 1 AS expected_chunks, actual_count
FROM max_per_doc
WHERE actual_count != max_idx + 1
LIMIT 10;
-- EXPECT: empty result (no gaps in chunk indices)
-- Non-empty means partial chunk storage (possible SAGA failure mid-pipeline)
