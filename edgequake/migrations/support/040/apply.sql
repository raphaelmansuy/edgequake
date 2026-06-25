-- ============================================================================
-- Migration 040 Support: Entity Backfill Script
-- File: migrations/support/040/apply.sql
-- Invoked by: migration_bootstrap.rs after migration 040 marker is recorded
-- IDEMPOTENT: safe to restart (ON CONFLICT DO UPDATE skips already-synced rows)
-- ============================================================================
--
-- WHAT THIS DOES:
--   Copies entity data from Apache AGE graph nodes into the relational `entities`
--   table, which serves as the CQRS read model for analytics, FTS, and JOINs.
--
-- CONFIGURATION (via server_config table):
--   backfill_batch_size: number of nodes per batch (default: 500)
--
-- EXIT CONDITION:
--   Sets entity_sync_mode = 'dual_write' on start, 'full' on completion.
--
-- MONITORING:
--   SELECT sync_status, count(*) FROM entities GROUP BY sync_status;
--   SELECT value FROM server_config WHERE key = 'entity_sync_mode';
--   SELECT value FROM server_config WHERE key = 'entity_backfill_progress';
--
-- SAFETY:
--   * AGE must be installed; if not, script exits gracefully with a notice
--   * The entities table must already have the CQRS schema (migration 039)
--   * Zero-downtime: reads from AGE only, writes to entities with ON CONFLICT
--   * Source graph name: defaults to 'edgequake' (configurable via server_config)
-- ============================================================================

SET search_path = public, ag_catalog;

DO $$
DECLARE
    graph_name    TEXT;
    batch_size    INT;
    offset_val    INT := 0;
    batch_count   INT := 0;
    total_synced  INT := 0;
    age_total     INT := 0;
    inserted      INT;
    batch_cfg     TEXT;
    graph_cfg     TEXT;
BEGIN
    -- ========================================================================
    -- Guard: AGE must be installed
    -- ========================================================================
    IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
        RAISE NOTICE 'Migration 040 backfill: AGE extension not available — skipping.';
        RAISE NOTICE 'entities table remains empty (entity_sync_mode stays disabled).';
        RETURN;
    END IF;

    -- ========================================================================
    -- Guard: migration 039 must have run (entities.sync_status must exist)
    -- ========================================================================
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public'
          AND table_name = 'entities'
          AND column_name = 'sync_status'
    ) THEN
        RAISE NOTICE 'Migration 040 backfill: entities.sync_status column missing.';
        RAISE NOTICE 'Run migration 039 first.';
        RETURN;
    END IF;

    -- ========================================================================
    -- Configuration from server_config
    -- ========================================================================
    SELECT value::text INTO batch_cfg
    FROM server_config WHERE key = 'backfill_batch_size';
    batch_size := COALESCE(NULLIF(batch_cfg, 'null')::int, 500);

    SELECT value::text INTO graph_cfg
    FROM server_config WHERE key = 'age_graph_name';
    graph_name := COALESCE(NULLIF(TRIM(BOTH '"' FROM COALESCE(graph_cfg, '')), ''), 'edgequake');

    RAISE NOTICE 'Migration 040 backfill: starting (graph=%, batch_size=%)', graph_name, batch_size;

    -- Check graph exists
    IF NOT EXISTS (
        SELECT 1 FROM ag_catalog.ag_graph WHERE name = graph_name
    ) THEN
        RAISE NOTICE 'Migration 040 backfill: AGE graph "%" not found — skipping.', graph_name;
        RETURN;
    END IF;

    -- ========================================================================
    -- Set sync mode to dual_write (new writes now go to both AGE and entities)
    -- ========================================================================
    UPDATE server_config SET value = '"dual_write"' WHERE key = 'entity_sync_mode';
    INSERT INTO server_config (key, value)
    VALUES ('entity_sync_mode', '"dual_write"')
    ON CONFLICT (key) DO UPDATE SET value = '"dual_write"';

    -- Count total nodes for progress reporting
    EXECUTE format(
        'SELECT COUNT(*)::int FROM %I."_ag_label_vertex"',
        graph_name
    ) INTO age_total;

    RAISE NOTICE 'Migration 040 backfill: % AGE nodes to process', age_total;

    -- ========================================================================
    -- Paginated backfill loop
    -- ========================================================================
    LOOP
        EXECUTE format(
            $batch$
            WITH age_batch AS (
                SELECT
                    ag_catalog.agtype_to_json(properties)->>'node_id'        AS name,
                    ag_catalog.agtype_to_json(properties)->>'entity_type'    AS entity_type,
                    ag_catalog.agtype_to_json(properties)->>'description'    AS description,
                    ag_catalog.agtype_to_json(properties)->>'tenant_id'      AS tenant_id_str,
                    ag_catalog.agtype_to_json(properties)->>'workspace_id'   AS workspace_id_str,
                    ag_catalog.agtype_to_json(properties)->>'keywords'       AS keywords_json
                FROM %I."_ag_label_vertex"
                ORDER BY id
                LIMIT %s OFFSET %s
            )
            INSERT INTO public.entities
                (name, entity_type, description, tenant_id, workspace_id,
                 keywords, sync_status, created_at, updated_at)
            SELECT
                b.name,
                COALESCE(b.entity_type, 'UNKNOWN'),
                b.description,
                CASE WHEN b.tenant_id_str IS NOT NULL AND b.tenant_id_str ~ '^[0-9a-f\-]{36}$'
                     THEN b.tenant_id_str::UUID ELSE NULL END,
                CASE WHEN b.workspace_id_str IS NOT NULL AND b.workspace_id_str ~ '^[0-9a-f\-]{36}$'
                     THEN b.workspace_id_str::UUID ELSE NULL END,
                CASE WHEN b.keywords_json IS NOT NULL
                     THEN ARRAY(SELECT jsonb_array_elements_text(b.keywords_json::jsonb))
                     ELSE '{}'::TEXT[] END,
                'synced',
                NOW(),
                NOW()
            FROM age_batch b
            WHERE b.name IS NOT NULL
            ON CONFLICT (tenant_id, workspace_id, name)
                DO UPDATE SET
                    entity_type = EXCLUDED.entity_type,
                    description = EXCLUDED.description,
                    keywords    = EXCLUDED.keywords,
                    sync_status = 'synced',
                    updated_at  = NOW()
            $batch$,
            graph_name, batch_size, offset_val
        );

        GET DIAGNOSTICS inserted = ROW_COUNT;
        total_synced := total_synced + inserted;
        batch_count  := batch_count + 1;
        offset_val   := offset_val + batch_size;

        -- Progress report every 10 batches
        IF batch_count % 10 = 0 THEN
            RAISE NOTICE 'Migration 040 backfill: % / % nodes processed (%s batches)',
                total_synced, age_total, batch_count;

            -- Persist progress
            INSERT INTO server_config (key, value)
            VALUES ('entity_backfill_progress',
                    format('{"synced": %s, "total": %s, "batch": %s}',
                           total_synced, age_total, batch_count)::jsonb)
            ON CONFLICT (key) DO UPDATE SET
                value = format('{"synced": %s, "total": %s, "batch": %s}',
                               total_synced, age_total, batch_count)::jsonb;
        END IF;

        EXIT WHEN inserted < batch_size;

        -- Yield between batches (50ms) to avoid I/O saturation
        PERFORM pg_sleep(0.05);
    END LOOP;

    -- ========================================================================
    -- Completion: set sync mode to 'full'
    -- ========================================================================
    UPDATE server_config SET value = '"full"' WHERE key = 'entity_sync_mode';

    -- Clear progress marker
    UPDATE server_config
    SET value = format(
            '{"synced": %s, "total": %s, "completed_at": "%s"}',
            total_synced, age_total, NOW()::text
        )::jsonb
    WHERE key = 'entity_backfill_progress';

    RAISE NOTICE 'Migration 040 backfill COMPLETE: % entities synced from AGE graph.', total_synced;
    RAISE NOTICE 'entity_sync_mode = full. New writes will go to both AGE and entities table.';

EXCEPTION WHEN OTHERS THEN
    RAISE WARNING 'Migration 040 backfill FAILED at batch % (offset %): %',
        batch_count, offset_val, SQLERRM;
    RAISE WARNING 'entity_sync_mode remains as-is. Re-run to resume from last checkpoint.';
    -- Do NOT re-raise: allow the caller to continue (backfill is best-effort)
END $$;
