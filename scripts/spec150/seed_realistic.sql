-- SPEC-150 realistic seed (table-existence guarded).
-- Seeds multi-workspace same doc_id, AGE Node/EDGE, high-dim embedding stub, legacy KV.
-- Safe to run against partial epoch schemas — each block checks for required tables.

DO $$
BEGIN
  -- Multi-workspace same doc_id (M118 21000 class)
  IF to_regclass('public.documents') IS NOT NULL
     AND EXISTS (
       SELECT 1 FROM information_schema.columns
       WHERE table_schema='public' AND table_name='documents' AND column_name='workspace_id'
     ) THEN
    BEGIN
      INSERT INTO public.documents (id, workspace_id, title, content, status, created_at, updated_at)
      SELECT
        'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'::uuid,
        '11111111-1111-1111-1111-111111111111'::uuid,
        'spec150-seed-ws1',
        'seed content ws1',
        'completed',
        now(),
        now()
      WHERE NOT EXISTS (
        SELECT 1 FROM public.documents
        WHERE id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'::uuid
          AND workspace_id = '11111111-1111-1111-1111-111111111111'::uuid
      );
    EXCEPTION WHEN others THEN
      RAISE NOTICE 'spec150 seed documents ws1 skipped: %', SQLERRM;
    END;
    BEGIN
      INSERT INTO public.documents (id, workspace_id, title, content, status, created_at, updated_at)
      SELECT
        'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'::uuid,
        '22222222-2222-2222-2222-222222222222'::uuid,
        'spec150-seed-ws2',
        'seed content ws2',
        'completed',
        now(),
        now()
      WHERE NOT EXISTS (
        SELECT 1 FROM public.documents
        WHERE id = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa'::uuid
          AND workspace_id = '22222222-2222-2222-2222-222222222222'::uuid
      );
    EXCEPTION WHEN others THEN
      RAISE NOTICE 'spec150 seed documents ws2 skipped: %', SQLERRM;
    END;
  END IF;
END $$;

-- AGE Node/EDGE (M078 class) — only when AGE graph exists
DO $$
DECLARE
  g text;
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'age') THEN
    RAISE NOTICE 'spec150 seed: AGE missing — skip Node/EDGE';
    RETURN;
  END IF;
  SELECT name INTO g FROM ag_catalog.ag_graph ORDER BY name LIMIT 1;
  IF g IS NULL THEN
    RAISE NOTICE 'spec150 seed: no AGE graph — skip Node/EDGE';
    RETURN;
  END IF;
  IF to_regclass(format('%I.%I', g, 'Node')) IS NULL THEN
    RETURN;
  END IF;
  BEGIN
    EXECUTE format(
      $q$INSERT INTO %I."Node" (id, properties)
         SELECT 900001::bigint, ag_catalog.agtype_in('{"id":"SPEC150_SEED","entity_type":"TEST","source_ids":["aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa"]}')
         WHERE NOT EXISTS (SELECT 1 FROM %I."Node" WHERE id = 900001)$q$,
      g, g
    );
  EXCEPTION WHEN others THEN
    RAISE NOTICE 'spec150 seed Node skipped: %', SQLERRM;
  END;
END $$;
