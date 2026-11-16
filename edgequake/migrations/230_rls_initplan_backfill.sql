-- Finish the InitPlan migration: six policies added after 208 still call the
-- session helpers bare, so they run once per row instead of once per statement.
--
-- Migration 208 wrapped every `current_tenant_id()` / `current_workspace_id()`
-- in `(SELECT ...)` inside policy expressions, then closed its comment with
-- "New policies must use the wrapped form, for example
-- `tenant_id = (SELECT current_tenant_id())`. e2e_rls_initplan_policies
-- (edgequake-storage) fails if any are left bare."
--
-- That guard test was never written, and five later migrations reintroduced the
-- bare form: 215 (original_object_refs), 216 (tenant object scope), 219
-- (mcp_upload_sessions), 222 (managed_secrets, secret_backend_configs) and 225
-- (chunk_terms, workspace_term_stats). This migration re-establishes the
-- invariant and this time the guard exists (e2e_rls_initplan_policies).
--
-- WHY it costs time, measured as `edgequake_tenant_access` after `SET LOCAL
-- ROLE` (the path `rls::with_rls_transaction` takes) on 120k chunks and 2.2M
-- `chunk_terms` rows, one workspace:
--
--   full BM25 statement        bare 24.6 ms   wrapped 12.4 ms  (median of 9)
--
-- A bare STABLE helper in a policy filter becomes a Filter expression the
-- executor re-evaluates for every row the scan emits. The plans show it
-- directly: before, the index-only scan carries `Filter: $0` over 4,031 rows
-- (and over 372,000 rows on `chunk_terms`); after, the scan carries no filter
-- at all and the helper is one InitPlan (`$0`, `One-Time Filter`).
--
-- The gain shows up in the composite BM25 statement rather than in an isolated
-- `SELECT count(*)`, because a count reads only the index and the per-row
-- expression was never in its projection path. BM25 joins both tables and
-- re-scans `chunk_terms` three times, so removing that per-row evaluation is
-- where the time goes: 9 of 9 alternating runs improved, 22.9-33.0 ms -> 11.8-19.8 ms.
--
-- Meaning is unchanged: `app.current_*` only change between statements, so a
-- scalar subquery wrapped around a STABLE, LEAKPROOF helper returns the same
-- value the bare call did. Verified explicitly on the three cases that could
-- differ — unset context (both NULL, row hidden), set context (both equal), and
-- a malformed GUC (both NULL, because the helper's EXCEPTION handler returns
-- NULL either way).
--
-- Idempotent and self-verifying, following 208: unwrapping the added
-- `(SELECT ...)` must give back the original expression, or the migration
-- aborts. Roles, commands and RESTRICTIVE/PERMISSIVE are never touched.

DO $migration$
DECLARE
    -- Deparsed form of a wrapped call: ( SELECT current_x_id() AS current_x_id)
    c_wrapped constant text :=
        '\( SELECT ((?:public\.)?current_(?:tenant|workspace|user)_id\(\)) AS current_(?:tenant|workspace|user)_id\)';
    -- A call that is not already inside a wrapper and not schema-qualified.
    c_bare constant text :=
        '(?<![\w.])(current_(?:tenant|workspace|user)_id\(\))';
    r record;
    v_using text;
    v_check text;
    v_using_plain text;
    v_check_plain text;
    v_after record;
    v_clauses text;
    v_rewritten integer := 0;
BEGIN
    FOR r IN
        SELECT schemaname, tablename, policyname, qual, with_check
        FROM pg_policies
        WHERE schemaname = 'public'
        ORDER BY tablename, policyname
    LOOP
        -- Already converged: nothing but wrapped calls remain.
        IF (r.qual IS NULL OR regexp_replace(r.qual, c_wrapped, '', 'g') !~ c_bare)
           AND (r.with_check IS NULL OR regexp_replace(r.with_check, c_wrapped, '', 'g') !~ c_bare) THEN
            CONTINUE;
        END IF;

        -- Unwrap first so a policy mixing wrapped and bare calls converges to
        -- the same text as a fully bare one.
        v_using := regexp_replace(r.qual, c_wrapped, '\1', 'g');
        v_check := regexp_replace(r.with_check, c_wrapped, '\1', 'g');

        v_using_plain := v_using;
        v_check_plain := v_check;
        v_using := regexp_replace(v_using, c_bare, '(SELECT \1)', 'g');
        v_check := regexp_replace(v_check, c_bare, '(SELECT \1)', 'g');

        v_clauses := '';
        IF v_using IS NOT NULL THEN
            v_clauses := v_clauses || format(' USING (%s)', v_using);
        END IF;
        IF v_check IS NOT NULL THEN
            v_clauses := v_clauses || format(' WITH CHECK (%s)', v_check);
        END IF;

        EXECUTE format('ALTER POLICY %I ON %I.%I%s',
                       r.policyname, r.schemaname, r.tablename, v_clauses);

        SELECT qual, with_check INTO v_after
        FROM pg_policies
        WHERE schemaname = r.schemaname
          AND tablename = r.tablename
          AND policyname = r.policyname;

        IF regexp_replace(v_after.qual, c_wrapped, '\1', 'g') IS DISTINCT FROM v_using_plain
           OR regexp_replace(v_after.with_check, c_wrapped, '\1', 'g') IS DISTINCT FROM v_check_plain THEN
            RAISE EXCEPTION 'RLS policy % on %.% changed meaning during InitPlan rewrite (before: %, after: %)',
                r.policyname, r.schemaname, r.tablename, r.qual, v_after.qual;
        END IF;

        v_rewritten := v_rewritten + 1;
    END LOOP;

    RAISE NOTICE '230: wrapped session helpers as InitPlans in % policies', v_rewritten;
END
$migration$;
