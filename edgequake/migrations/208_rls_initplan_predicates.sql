-- Evaluate the session-context helpers once per statement, not once per row.
--
-- Every tenant policy calls current_tenant_id(), current_workspace_id() or
-- current_user_id() directly. They are STABLE plpgsql functions with an
-- EXCEPTION block, so the planner cannot inline them and each call opens a
-- subtransaction. A STABLE call in a filter runs for every row it filters.
-- Counting one workspace of 66,682 graph nodes as edgequake_tenant_access:
--
--     bare call            184 ms   Filter: current_workspace_id() IS NULL OR ...
--     (SELECT fn())         13 ms   InitPlan, evaluated once
--     no policy predicate    6 ms
--
-- Wrapping the call as a scalar subquery turns it into an InitPlan. The value
-- is fixed for the statement either way (app.current_* only change between
-- statements), so the policy means exactly what it meant before.
--
-- This rewrites policy text in place. It does not change roles, commands, or
-- RESTRICTIVE/PERMISSIVE. It is idempotent and verifies each rewrite: removing
-- the added wrappers from the new expression must give back the old one, or
-- the whole migration aborts.
--
-- New policies must use the wrapped form, for example:
--     tenant_id = (SELECT current_tenant_id())
-- e2e_rls_initplan_policies (edgequake-storage) fails if any are left bare.

DO $migration$
DECLARE
    -- Deparsed form of a wrapped call: ( SELECT current_x_id() AS current_x_id)
    c_wrapped constant text :=
        '\( SELECT ((?:public\.)?current_(?:tenant|workspace|user)_id\(\)) AS current_(?:tenant|workspace|user)_id\)';
    -- A call that is not already inside a wrapper or schema-qualified.
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

        -- Unwrap first so a policy that mixes wrapped and bare calls
        -- converges to the same text as a fully bare one.
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

    RAISE NOTICE '208: wrapped session helpers as InitPlans in % policies', v_rewritten;
END
$migration$;
