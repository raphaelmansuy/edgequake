-- SPEC-042-E E-02 — AGE 1.7 graph label RLS (SSOT)
--
-- Invoked when EDGEQUAKE_AGE_RLS=true and AGE extversion >= 1.7.0.
-- Policies filter on properties->>'tenant_id' vs session edgequake.tenant_id.

DO $$
DECLARE
    v_graph text;
    v_age text;
    v_vertex text;
    v_edge text;
    policies int := 0;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'Migration 081 apply — AGE not installed, skipping';
        RETURN;
    END IF;

    SELECT extversion INTO v_age FROM pg_extension WHERE extname = 'age';
    IF v_age IS NULL OR string_to_array(v_age, '.')::int[] < string_to_array('1.7.0', '.')::int[] THEN
        RAISE NOTICE 'Migration 081 apply — AGE % < 1.7.0, skipping RLS', v_age;
        RETURN;
    END IF;

    FOR v_graph IN SELECT name FROM ag_catalog.ag_graph ORDER BY name
    LOOP
        v_vertex := format('%I._ag_label_vertex', v_graph);
        IF to_regclass(v_vertex) IS NOT NULL THEN
            EXECUTE format('ALTER TABLE %s ENABLE ROW LEVEL SECURITY', v_vertex);
            EXECUTE format('ALTER TABLE %s FORCE ROW LEVEL SECURITY', v_vertex);
            IF NOT EXISTS (
                SELECT 1 FROM pg_policies
                WHERE schemaname = v_graph AND tablename = '_ag_label_vertex'
                  AND policyname = 'edgequake_tenant_isolation_vertex'
            ) THEN
                EXECUTE format(
                    'CREATE POLICY edgequake_tenant_isolation_vertex ON %s
                     USING (
                       coalesce(ag_catalog.agtype_to_json(properties)->>''tenant_id'', '''') = coalesce(
                         current_setting(''edgequake.tenant_id'', true), ''''
                       )
                       OR current_setting(''edgequake.tenant_id'', true) IS NULL
                       OR current_setting(''edgequake.tenant_id'', true) = ''''
                     )',
                    v_vertex
                );
                policies := policies + 1;
            END IF;
        END IF;

        v_edge := format('%I._ag_label_edge', v_graph);
        IF to_regclass(v_edge) IS NOT NULL THEN
            EXECUTE format('ALTER TABLE %s ENABLE ROW LEVEL SECURITY', v_edge);
            EXECUTE format('ALTER TABLE %s FORCE ROW LEVEL SECURITY', v_edge);
            IF NOT EXISTS (
                SELECT 1 FROM pg_policies
                WHERE schemaname = v_graph AND tablename = '_ag_label_edge'
                  AND policyname = 'edgequake_tenant_isolation_edge'
            ) THEN
                EXECUTE format(
                    'CREATE POLICY edgequake_tenant_isolation_edge ON %s
                     USING (
                       coalesce(ag_catalog.agtype_to_json(properties)->>''tenant_id'', '''') = coalesce(
                         current_setting(''edgequake.tenant_id'', true), ''''
                       )
                       OR current_setting(''edgequake.tenant_id'', true) IS NULL
                       OR current_setting(''edgequake.tenant_id'', true) = ''''
                     )',
                    v_edge
                );
                policies := policies + 1;
            END IF;
        END IF;
    END LOOP;

    RAISE NOTICE 'Migration 081 apply complete — ensured % AGE RLS policies', policies;
END $$;
