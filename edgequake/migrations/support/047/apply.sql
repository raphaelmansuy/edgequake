-- SPEC-027 Migration 047 — workspace document KV index backfill (SSOT)
--
-- Adds `wsdoc:{workspace_id}:{document_id}` pointer keys for every final
-- `{document_id}-metadata` row across all `eq_*_kv` tables. Enables O(workspace)
-- prefix scans instead of global `-metadata` suffix scans.

DO $$
DECLARE
    tbl record;
    r record;
    doc_id text;
    ws_id text;
    tenant_id text;
    idx_key text;
    idx_val jsonb;
    backfilled int := 0;
BEGIN
    FOR tbl IN
        SELECT tablename
        FROM pg_tables
        WHERE schemaname = 'public'
          AND tablename LIKE 'eq\_%\_kv' ESCAPE '\'
          AND tablename NOT LIKE '%\_stats' ESCAPE '\'
    LOOP
        FOR r IN EXECUTE format(
            'SELECT key, value FROM %I
             WHERE key LIKE ''%%-metadata''
               AND key NOT LIKE ''staging:%%''',
            tbl.tablename
        ) LOOP
            doc_id := left(r.key, length(r.key) - 9);
            IF doc_id IS NULL OR doc_id = '' THEN
                CONTINUE;
            END IF;

            ws_id := coalesce(r.value->>'workspace_id', 'default');
            tenant_id := coalesce(r.value->>'tenant_id', 'default');
            idx_key := format('wsdoc:%s:%s', ws_id, doc_id);
            idx_val := jsonb_build_object(
                'metadata_key', r.key,
                'document_id', doc_id,
                'workspace_id', ws_id,
                'tenant_id', tenant_id
            );

            EXECUTE format(
                'INSERT INTO %I (key, value) VALUES ($1, $2)
                 ON CONFLICT (key) DO UPDATE
                 SET value = EXCLUDED.value, updated_at = NOW()',
                tbl.tablename
            ) USING idx_key, idx_val;

            backfilled := backfilled + 1;
        END LOOP;
    END LOOP;

    RAISE NOTICE 'Migration 047 apply complete — wsdoc index entries upserted: %', backfilled;
END $$;
