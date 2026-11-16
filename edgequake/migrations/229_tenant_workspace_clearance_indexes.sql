-- Give the tenant/workspace scope indexes the clearance columns they are
-- always filtered with, so RLS can answer from the index alone.
--
-- Migration 220 added a restrictive clearance policy to eight tables. For
-- chunks and documents that policy reads `classification_rank`,
-- `compartment_ids` and `uploaded_by` on every surviving row while the scan
-- itself is driven by `(tenant_id, workspace_id)`. The composite index covers
-- the scan but not the policy, so each candidate row is fetched from the heap
-- only to be discarded. Measured as `edgequake_tenant_access` with
-- `SET LOCAL ROLE`, 120,000 chunks / 1,200 documents in one workspace:
--
--   chunks count        Bitmap Heap Scan, 18,000 heap blocks   59 ms
--   chunks count        Index Only Scan, 0 heap fetches         6 ms
--   documents count     Bitmap Heap Scan                        1.7 ms
--   documents count     Index Only Scan                         0.3 ms
--
-- `INCLUDE` puts those columns in the leaf, so the same index is index-only
-- for the policy. The keys are unchanged, so every existing equality and
-- range use of (tenant_id, workspace_id) plans exactly as before.
--
-- Each statement replaces the existing index instead of adding one: the table
-- keeps the same number of indexes to maintain, and the wider leaf is the only
-- cost. On this corpus the chunks index grows 1.6 MB -> 6.9 MB and the
-- documents index 120 kB -> 248 kB.
--
-- Also drops two index pairs that are byte-for-byte duplicate definitions, so
-- every write maintained both for no read benefit:
--
--   documents.idx_documents_status_v2  duplicates idx_documents_status
--                                      (both are btree(status), both non-unique,
--                                      both unpartial; 001 and 017 created them)
--   conversation_history.idx_conversation_history_conversation_id
--                                      duplicates unique_conversation_message
--                                      (both btree(conversation_id, message_index),
--                                      same order, no predicate)
--
-- Verified the status lookup still plans an index scan afterwards, and that the
-- UNIQUE constraint on conversation_history still owns its index.

-- ── chunks: scope index becomes index-only for the clearance policy ─────────
DROP INDEX IF EXISTS public.idx_chunks_tenant_workspace;
CREATE INDEX IF NOT EXISTS idx_chunks_tenant_workspace
    ON public.chunks (tenant_id, workspace_id)
    INCLUDE (classification_rank, compartment_ids, uploaded_by);

-- ── documents: same treatment ──────────────────────────────────────────────
DROP INDEX IF EXISTS public.idx_documents_tenant_workspace;
CREATE INDEX IF NOT EXISTS idx_documents_tenant_workspace
    ON public.documents (tenant_id, workspace_id)
    INCLUDE (classification_rank, compartment_ids, uploaded_by);

-- ── exact duplicate index pairs ────────────────────────────────────────────
DROP INDEX IF EXISTS public.idx_documents_status_v2;
DROP INDEX IF EXISTS public.idx_conversation_history_conversation_id;

ANALYZE public.chunks;
ANALYZE public.documents;
ANALYZE public.conversation_history;
