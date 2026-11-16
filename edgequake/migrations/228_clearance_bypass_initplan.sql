-- Evaluate the clearance bypass once per statement instead of once per row.
--
-- The clearance policy (migration 220) puts three disjuncts inside one
-- restrictive predicate:
--
--     clearance_covers(rank, compartments)  -- a per-row function
--     OR (uploaded_by = current_user_id() AND rank = 5)
--
-- `clearance_covers` is LEAKPROOF and STABLE, but it is not a constant: its
-- first two disjuncts read only session GUCs and one workspace settings row,
-- so for the whole statement they are the same value. Because those terms sit
-- inside the per-row function, the planner must call the function for every
-- candidate row. A scalar subquery cannot be hoisted out of it.
--
-- Measured as `edgequake_tenant_access` with `SET LOCAL ROLE` (the exact path
-- `rls::with_rls_transaction` takes), on 120,000 chunks / 36,000 in one
-- workspace, enforcement off (the common case):
--
--   counting one workspace          before  598 ms   after   6 ms
--   chunk FTS page (LIMIT 20)       before  473 ms   after  82 ms
--   document page (LIMIT 50)        before   13 ms   after   1 ms
--   document count                  before   12 ms   after  0.3 ms
--
-- With the policy forced to literal TRUE the same count is 4 ms, so the
-- remaining 2 ms is the InitPlan itself, not per-row work.
--
-- The fix splits the predicate at the point where row-independence ends:
--
--   clearance_bypass()      row-independent: break-glass, enforcement off
--   clearance_rank_covers() per row: rank and compartment subset
--   clearance_covers()      public API, still bypass() OR rank_covers()
--
-- Policies use `(SELECT clearance_bypass()) OR clearance_rank_covers(...)`.
-- A `(SELECT ...)` around a row-independent function becomes an InitPlan, the
-- same technique migration 208 applied to current_tenant_id(). The per-row
-- function is now a separate disjunct that only runs when the statement is
-- enforcing, and then only while it can still change the answer.
--
-- Semantics are unchanged, by construction and by test: the original predicate
-- was `bypass OR per_row`, and the new one is that same disjunction with the
-- row-independent half wrapped as a subquery. `clearance_covers()` keeps its
-- name, signature and meaning for its callers (the graph contribution reads in
-- `graph/plain/{read,search}.rs`), so application SQL needs no change.
--
-- Expand-only: three CREATE OR REPLACE FUNCTION plus policy text. No column,
-- index, constraint or row is touched, and the migration is idempotent.

-- ── Row-independent half ───────────────────────────────────────────────────
-- Break-glass, or this workspace is not enforcing. Reads session GUCs and at
-- most one `workspace_clearance_settings` row; never a column of the row being
-- filtered, which is what lets the planner evaluate it once.
CREATE OR REPLACE FUNCTION public.clearance_bypass()
RETURNS boolean
LANGUAGE sql
STABLE
LEAKPROOF
SET search_path = pg_catalog, public
AS $$
    SELECT
        (SELECT current_setting('app.clearance_break_glass', true)) = 'on'
        OR (
            (SELECT current_setting('app.clearance_enforced', true)) IS DISTINCT FROM 'on'
            AND COALESCE(
                (SELECT s.enforcement
                 FROM public.workspace_clearance_settings s
                 WHERE s.workspace_id = (SELECT public.current_workspace_id())),
                false
            ) = false
        )
$$;

-- ── Per-row half ───────────────────────────────────────────────────────────
-- Rank and compartment coverage only. This is the second and third disjunct of
-- the original function, verbatim, with the bypass disjunct removed.
CREATE OR REPLACE FUNCTION public.clearance_rank_covers(
    doc_rank smallint,
    doc_compartments uuid[]
)
RETURNS boolean
LANGUAGE sql
STABLE
LEAKPROOF
SET search_path = pg_catalog, public
AS $$
    SELECT
        doc_rank IS NOT NULL
        AND (SELECT public.current_clearance_rank()) IS NOT NULL
        AND doc_rank <= (SELECT public.current_clearance_rank())
        AND (
            (
                COALESCE(cardinality(doc_compartments), 0) = 0
                AND (SELECT current_setting('app.clearance_allow_empty', true)) IS DISTINCT FROM 'off'
            )
            OR (
                COALESCE(cardinality(doc_compartments), 0) > 0
                AND doc_compartments <@ (SELECT public.current_clearance_compartments())
            )
        )
$$;

-- ── Public API: same name, same meaning, now composed from the two halves ───
-- Callers outside a policy (graph contribution reads) keep working unchanged.
-- The nested `(SELECT ...)` keeps each half an InitPlan there as well.
CREATE OR REPLACE FUNCTION public.clearance_covers(
    doc_rank smallint,
    doc_compartments uuid[]
)
RETURNS boolean
LANGUAGE sql
STABLE
LEAKPROOF
SET search_path = pg_catalog, public
AS $$
    SELECT (SELECT public.clearance_bypass())
        OR (SELECT public.clearance_rank_covers(doc_rank, doc_compartments))
$$;

-- ── Rewrite the policies to expose the row-independent term ────────────────
-- Same tables, same roles, same RESTRICTIVE/FOR ALL. Only the USING and
-- WITH CHECK text changes: `clearance_covers(...)` is replaced by
-- `(SELECT clearance_bypass()) OR clearance_rank_covers(...)`, which is the
-- same predicate with its row-independent half hoisted.
DO $migration$
DECLARE
    tbl text;
    predicate text;
BEGIN
    FOREACH tbl IN ARRAY ARRAY[
        'documents', 'chunks', 'document_originals', 'pdf_documents',
        'document_mm_assets', 'document_pages', 'tasks', 'graph_contributions'
    ] LOOP
        IF to_regclass(format('public.%I', tbl)) IS NULL THEN
            CONTINUE;
        END IF;
        IF NOT EXISTS (
            SELECT 1 FROM pg_attribute
            WHERE attrelid = to_regclass(format('public.%I', tbl))
              AND attname = 'classification_rank'
              AND NOT attisdropped
        ) THEN
            CONTINUE;
        END IF;

        -- The uploader clause is unchanged. It stays last so the cheap
        -- InitPlan disjunct and the rank check short-circuit before it.
        predicate := format(
            '(SELECT public.clearance_bypass())'
            ' OR (SELECT public.clearance_rank_covers(classification_rank, compartment_ids))'
            ' OR (uploaded_by = (SELECT public.current_user_id()) AND classification_rank = 5)'
        );
        IF tbl = 'tasks' THEN
            predicate := format('document_id IS NULL OR (%s)', predicate);
        END IF;

        -- Recreate rather than ALTER: the expression is replaced wholesale and
        -- DROP+CREATE is the form migration 220 already used, so a re-run
        -- converges to identical policy text.
        EXECUTE format('DROP POLICY IF EXISTS clearance_guard ON public.%I', tbl);
        EXECUTE format(
            'CREATE POLICY clearance_guard ON public.%I AS RESTRICTIVE FOR ALL TO edgequake_tenant_access'
            ' USING ((%s)) WITH CHECK ((%s))',
            tbl, predicate, predicate
        );
    END LOOP;
END $migration$;
