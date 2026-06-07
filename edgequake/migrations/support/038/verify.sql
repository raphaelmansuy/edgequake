-- SPEC-006 Migration 038 — Post-apply verification (read-only)
-- Confirms expected indexes exist per AGE graph.
--
-- Usage:
--   edgequake/scripts/migrations/apply_038.sh --verify

DO $$
DECLARE
    graph_name text;
    graph_schema text;
    idx_prefix text;
    missing int := 0;
    expected text;
    found boolean;
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'AGE not installed — nothing to verify';
        RETURN;
    END IF;

    RAISE NOTICE '=== Migration 038 index verification ===';

    FOR graph_name IN SELECT name FROM ag_catalog.ag_graph LOOP
        graph_schema := graph_name;
        idx_prefix := replace(graph_name, '.', '_');

        IF to_regclass(format('%I._ag_label_vertex', graph_schema)) IS NULL
           AND to_regclass(format('%I._ag_label_edge', graph_schema)) IS NULL THEN
            RAISE NOTICE 'Skip graph % — no AGE label tables', graph_name;
            CONTINUE;
        END IF;

        RAISE NOTICE '--- Graph: % ---', graph_name;

        FOR expected IN
            SELECT unnest(ARRAY[
                format('idx_%s_vertex_source_id', idx_prefix),
                format('idx_%s_vertex_source_ids_gin', idx_prefix),
                format('idx_%s_edge_source_ids_gin', idx_prefix)
            ])
        LOOP
            SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE schemaname = graph_schema AND indexname = expected
            ) INTO found;

            IF found THEN
                RAISE NOTICE '  ✓ %', expected;
            ELSE
                -- edge index optional when edge table missing
                IF expected LIKE '%edge_source_ids%' AND to_regclass(format('%I._ag_label_edge', graph_schema)) IS NULL THEN
                    RAISE NOTICE '  ~ % (skipped — no edge table)', expected;
                ELSIF expected LIKE '%vertex%' AND to_regclass(format('%I._ag_label_vertex', graph_schema)) IS NULL THEN
                    RAISE NOTICE '  ~ % (skipped — no vertex table)', expected;
                ELSE
                    RAISE WARNING '  ✗ MISSING: %', expected;
                    missing := missing + 1;
                END IF;
            END IF;
        END LOOP;
    END LOOP;

    IF missing > 0 THEN
        RAISE EXCEPTION 'Migration 038 verification failed: % missing index(es)', missing;
    END IF;

    RAISE NOTICE 'Verification passed — all required indexes present';
END $$;
