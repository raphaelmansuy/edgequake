-- SPEC-164 measurement harness (scratch only).
-- Schema: spec164. AGE graph: spec164_age.
-- Do not run against production graph names.

CREATE EXTENSION IF NOT EXISTS age CASCADE;
LOAD 'age';
SET search_path = ag_catalog, "$user", public;

CREATE SCHEMA IF NOT EXISTS spec164;

-- ---------------------------------------------------------------------------
-- Result sink
-- ---------------------------------------------------------------------------
DROP TABLE IF EXISTS spec164.bench_runs CASCADE;
CREATE TABLE spec164.bench_runs (
  id          bigserial PRIMARY KEY,
  scale_id    text NOT NULL,
  arm_id      text NOT NULL,
  op_id       text NOT NULL,
  run_ix      int  NOT NULL,
  elapsed_ms  double precision NOT NULL,
  rows_out    bigint,
  notes       text,
  recorded_at timestamptz NOT NULL DEFAULT clock_timestamp()
);

DROP TABLE IF EXISTS spec164.correctness CASCADE;
CREATE TABLE spec164.correctness (
  scale_id text NOT NULL,
  start_id text NOT NULL,
  arm_a_hash text,
  arm_b_hash text,
  arm_c_hash text,
  ok boolean NOT NULL
);

-- ---------------------------------------------------------------------------
-- Plain adjacency DDL factory
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION spec164.ensure_plain_tables(p_suffix text)
RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  EXECUTE format($f$
    CREATE TABLE IF NOT EXISTS spec164.nodes_%s (
      node_id      text PRIMARY KEY,
      tenant_id    text NOT NULL,
      workspace_id text NOT NULL,
      entity_type  text,
      properties   jsonb NOT NULL DEFAULT '{}'::jsonb
    )$f$, p_suffix);
  EXECUTE format($f$
    CREATE TABLE IF NOT EXISTS spec164.edges_%s (
      source_id    text NOT NULL,
      target_id    text NOT NULL,
      rel_type     text NOT NULL DEFAULT '',
      tenant_id    text NOT NULL,
      workspace_id text NOT NULL,
      weight       double precision,
      properties   jsonb NOT NULL DEFAULT '{}'::jsonb,
      PRIMARY KEY (source_id, target_id, rel_type)
    )$f$, p_suffix);
  EXECUTE format(
    'CREATE INDEX IF NOT EXISTS idx_nodes_%s_tw ON spec164.nodes_%s (tenant_id, workspace_id)',
    p_suffix, p_suffix);
  EXECUTE format(
    'CREATE INDEX IF NOT EXISTS idx_edges_%s_src ON spec164.edges_%s (source_id)',
    p_suffix, p_suffix);
  EXECUTE format(
    'CREATE INDEX IF NOT EXISTS idx_edges_%s_tgt ON spec164.edges_%s (target_id)',
    p_suffix, p_suffix);
  EXECUTE format(
    'CREATE INDEX IF NOT EXISTS idx_edges_%s_tw ON spec164.edges_%s (tenant_id, workspace_id)',
    p_suffix, p_suffix);
  -- Search support (OP-164-10). Lineage GIN is built after the bulk load.
  EXECUTE format(
    'CREATE INDEX IF NOT EXISTS idx_nodes_%s_nid_lower ON spec164.nodes_%s (lower(node_id) text_pattern_ops)',
    p_suffix, p_suffix);
END;
$$;

-- Lineage GIN for OP-164-12. Build this AFTER the node insert.
-- A GIN created first absorbs the 200k-row load into its pending list, and
-- `properties->'source_ids' ?|` then scans that list (~5× the AGE index, which
-- load_scale builds with CREATE INDEX on the finished table). jsonb_ops and
-- gin_pending_list_limit match the product plain indexes (migration 170).
CREATE OR REPLACE FUNCTION spec164.ensure_lineage_gin(p_suffix text)
RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
  idx regclass;
BEGIN
  EXECUTE format(
    'CREATE INDEX IF NOT EXISTS idx_nodes_%s_src_gin ON spec164.nodes_%s USING GIN ((properties->''source_ids'') jsonb_ops) WITH (gin_pending_list_limit = 256)',
    p_suffix, p_suffix);
  idx := format('spec164.idx_nodes_%s_src_gin', p_suffix)::regclass;
  PERFORM gin_clean_pending_list(idx);
END;
$$;

CREATE OR REPLACE FUNCTION spec164.drop_plain_tables(p_suffix text)
RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  EXECUTE format('DROP TABLE IF EXISTS spec164.edges_%s CASCADE', p_suffix);
  EXECUTE format('DROP TABLE IF EXISTS spec164.nodes_%s CASCADE', p_suffix);
END;
$$;

-- ---------------------------------------------------------------------------
-- AGE graph ensure / drop
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION spec164.ensure_age_graph()
RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
  exists_g boolean;
BEGIN
  LOAD 'age';
  PERFORM set_config('search_path', 'ag_catalog, "$user", public', true);
  SELECT EXISTS (
    SELECT 1 FROM ag_catalog.ag_graph WHERE name = 'spec164_age'
  ) INTO exists_g;
  IF exists_g THEN
    PERFORM ag_catalog.drop_graph('spec164_age', true);
  END IF;
  PERFORM ag_catalog.create_graph('spec164_age');
  PERFORM ag_catalog.create_vlabel('spec164_age', 'Node');
  PERFORM ag_catalog.create_elabel('spec164_age', 'EDGE');

  EXECUTE 'ALTER TABLE spec164_age."Node" ADD COLUMN IF NOT EXISTS eq_node_id text';
  EXECUTE 'ALTER TABLE spec164_age."EDGE" ADD COLUMN IF NOT EXISTS eq_source_id text';
  EXECUTE 'ALTER TABLE spec164_age."EDGE" ADD COLUMN IF NOT EXISTS eq_target_id text';
  EXECUTE 'ALTER TABLE spec164_age."EDGE" ADD COLUMN IF NOT EXISTS eq_rel_type text';

  EXECUTE 'CREATE UNIQUE INDEX IF NOT EXISTS idx_spec164_eq_node ON spec164_age."Node" (eq_node_id)';
  EXECUTE 'CREATE INDEX IF NOT EXISTS idx_spec164_eq_src ON spec164_age."EDGE" (eq_source_id)';
  EXECUTE 'CREATE INDEX IF NOT EXISTS idx_spec164_eq_tgt ON spec164_age."EDGE" (eq_target_id)';
  EXECUTE 'CREATE UNIQUE INDEX IF NOT EXISTS idx_spec164_eq_edge_uk ON spec164_age."EDGE" (eq_source_id, eq_target_id, eq_rel_type)';
  EXECUTE 'CREATE INDEX IF NOT EXISTS idx_spec164_edge_start_txt ON spec164_age."EDGE" ((start_id::text))';
  EXECUTE 'CREATE INDEX IF NOT EXISTS idx_spec164_edge_end_txt ON spec164_age."EDGE" ((end_id::text))';
END;
$$;

-- ---------------------------------------------------------------------------
-- Load SCALE into plain + AGE (same logical graph)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION spec164.load_scale(
  p_suffix text,
  p_n_nodes bigint,
  p_n_edges bigint,
  p_hub_degree int DEFAULT 0
) RETURNS void
LANGUAGE plpgsql AS $$
DECLARE
  t0 timestamptz := clock_timestamp();
BEGIN
  PERFORM spec164.drop_plain_tables(p_suffix);
  PERFORM spec164.ensure_plain_tables(p_suffix);
  PERFORM spec164.ensure_age_graph();

  -- Plain nodes (include source_ids for lineage GIN — Q-GRAPH-017)
  EXECUTE format($f$
    INSERT INTO spec164.nodes_%s (node_id, tenant_id, workspace_id, entity_type, properties)
    SELECT 'n' || g, 't164', 'w164', 'ENTITY',
           jsonb_build_object(
             'node_id', 'n' || g,
             'description', 'node ' || g,
             'source_ids', jsonb_build_array(
               'doc:' || ((g - 1) / 100),
               'doc:' || ((g - 1) / 100) || '_' || ((g - 1) %% 100)
             )
           )
    FROM generate_series(1, %s) AS g
  $f$, p_suffix, p_n_nodes);

  PERFORM spec164.ensure_lineage_gin(p_suffix);

  -- Plain edges: ring + random long-range (deterministic via hash).
  -- WHY abs(hashtext): PostgreSQL hashtext can be negative; bare %% produced n-1401 orphans.
  EXECUTE format($f$
    INSERT INTO spec164.edges_%s (source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
    SELECT
      'n' || ((g - 1) %% %s + 1),
      'n' || (CASE
        WHEN g <= %s THEN (g %% %s) + 1
        ELSE (mod(abs(hashtext('e' || g::text)), %s::int) + 1)
      END),
      'REL',
      't164', 'w164', 1.0,
      jsonb_build_object('relation_type', 'REL')
    FROM generate_series(1, %s) AS g
    ON CONFLICT DO NOTHING
  $f$, p_suffix, p_n_nodes, p_n_nodes, p_n_nodes, p_n_nodes, p_n_edges);

  IF p_hub_degree > 0 THEN
    -- Attach hub edges from n1 to n2..n(hub+1)
    EXECUTE format($f$
      INSERT INTO spec164.edges_%s (source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
      SELECT 'n1', 'n' || g, 'HUB', 't164', 'w164', 1.0,
             jsonb_build_object('relation_type', 'HUB')
      FROM generate_series(2, %s + 1) AS g
      ON CONFLICT DO NOTHING
    $f$, p_suffix, p_hub_degree);
  END IF;

  -- AGE nodes (native insert mirroring product + source_ids)
  INSERT INTO spec164_age."Node" (id, properties, eq_node_id)
  SELECT
    public.eq_next_node_id('spec164_age'),
    format(
      '{"node_id": %s, "tenant_id": "t164", "workspace_id": "w164", "entity_type": "ENTITY", "description": "node %s", "source_ids": ["doc:%s", "doc:%s_%s"]}',
      to_json('n' || g)::text,
      g,
      (g - 1) / 100,
      (g - 1) / 100,
      (g - 1) % 100
    )::ag_catalog.agtype,
    'n' || g
  FROM generate_series(1, p_n_nodes) AS g;

  -- Lineage GIN on AGE child (expression over jsonb cast) — Q-GRAPH-017 shape
  EXECUTE $ix$
    CREATE INDEX IF NOT EXISTS idx_spec164_age_src_gin
    ON spec164_age."Node"
    USING GIN ((ag_catalog.agtype_to_json(properties)::jsonb->'source_ids'))
  $ix$;
  EXECUTE $ix$
    CREATE INDEX IF NOT EXISTS idx_spec164_age_nid_lower
    ON spec164_age."Node"
    ((lower(ag_catalog.agtype_to_json(properties)->>'node_id')))
  $ix$;

  -- AGE edges: resolve endpoints via eq_node_id
  EXECUTE format($f$
    INSERT INTO spec164_age."EDGE" (id, start_id, end_id, properties, eq_source_id, eq_target_id, eq_rel_type)
    SELECT
      public.eq_next_edge_id('spec164_age'),
      ns.id,
      nt.id,
      format(
        '{"source_id": %%s, "target_id": %%s, "relation_type": "REL", "tenant_id": "t164", "workspace_id": "w164"}',
        to_json(e.source_id)::text,
        to_json(e.target_id)::text
      )::ag_catalog.agtype,
      e.source_id,
      e.target_id,
      e.rel_type
    FROM spec164.edges_%s e
    JOIN spec164_age."Node" ns ON ns.eq_node_id = e.source_id
    JOIN spec164_age."Node" nt ON nt.eq_node_id = e.target_id
  $f$, p_suffix);

  ANALYZE;
  RAISE NOTICE 'load_scale % nodes=% edges_plain=% hub=% elapsed_ms=%',
    p_suffix, p_n_nodes, p_n_edges, p_hub_degree,
    EXTRACT(EPOCH FROM (clock_timestamp() - t0)) * 1000;
END;
$$;

-- ---------------------------------------------------------------------------
-- Ops: plain
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION spec164.plain_point_get(p_suffix text, p_id text)
RETURNS jsonb
LANGUAGE plpgsql AS $$
DECLARE r jsonb;
BEGIN
  EXECUTE format(
    'SELECT to_jsonb(n) FROM spec164.nodes_%s n WHERE node_id = $1',
    p_suffix
  ) INTO r USING p_id;
  RETURN r;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_incident(p_suffix text, p_ids text[])
RETURNS SETOF text
LANGUAGE plpgsql AS $$
BEGIN
  RETURN QUERY EXECUTE format($f$
    SELECT e.source_id || '->' || e.target_id
    FROM spec164.edges_%s e
    WHERE e.source_id = ANY($1) OR e.target_id = ANY($1)
  $f$, p_suffix) USING p_ids;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_degree_batch(p_suffix text, p_ids text[])
RETURNS TABLE(node_id text, deg bigint)
LANGUAGE plpgsql AS $$
BEGIN
  RETURN QUERY EXECUTE format($f$
    SELECT n.node_id,
           COUNT(e.*)::bigint AS deg
    FROM unnest($1) AS u(node_id)
    JOIN spec164.nodes_%s n ON n.node_id = u.node_id
    LEFT JOIN spec164.edges_%s e
      ON e.source_id = n.node_id OR e.target_id = n.node_id
    GROUP BY n.node_id
  $f$, p_suffix, p_suffix) USING p_ids;
END;
$$;

-- Iterative BFS (product shape): depth k, cap max_nodes
CREATE OR REPLACE FUNCTION spec164.plain_bfs(
  p_suffix text,
  p_start text,
  p_depth int,
  p_max_nodes int
) RETURNS text[]
LANGUAGE plpgsql AS $$
DECLARE
  visited text[] := ARRAY[p_start];
  frontier text[] := ARRAY[p_start];
  nxt text[];
  d int;
BEGIN
  FOR d IN 1..p_depth LOOP
    EXIT WHEN frontier IS NULL OR cardinality(visited) >= p_max_nodes;
    EXECUTE format($f$
      SELECT COALESCE(array_agg(DISTINCT x ORDER BY x), ARRAY[]::text[])
      FROM (
        SELECT CASE WHEN e.source_id = ANY($1) THEN e.target_id ELSE e.source_id END AS x
        FROM spec164.edges_%s e
        WHERE e.source_id = ANY($1) OR e.target_id = ANY($1)
      ) s
      WHERE x <> ALL($2)
    $f$, p_suffix)
    INTO nxt
    USING frontier, visited;

    nxt := nxt[1:greatest(p_max_nodes - cardinality(visited), 0)];
    visited := visited || nxt;
    frontier := nxt;
  END LOOP;
  RETURN visited[1:p_max_nodes];
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_level_cte(
  p_suffix text,
  p_start text,
  p_depth int,
  p_max_nodes int
) RETURNS text[]
LANGUAGE plpgsql AS $$
DECLARE ids text[];
BEGIN
  EXECUTE format($f$
    WITH RECURSIVE walk AS (
      SELECT $1::text AS node_id, 0 AS depth
      UNION
      SELECT CASE WHEN e.source_id = w.node_id THEN e.target_id ELSE e.source_id END,
             w.depth + 1
      FROM walk w
      JOIN spec164.edges_%s e
        ON e.source_id = w.node_id OR e.target_id = w.node_id
      WHERE w.depth < $2
    )
    SELECT COALESCE(array_agg(node_id ORDER BY node_id), ARRAY[]::text[])
    FROM (
      SELECT DISTINCT node_id FROM walk LIMIT $3
    ) s
  $f$, p_suffix)
  INTO ids
  USING p_start, p_depth, p_max_nodes;
  RETURN ids;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_path_cte(
  p_suffix text,
  p_start text,
  p_depth int,
  p_max_nodes int
) RETURNS text[]
LANGUAGE plpgsql AS $$
DECLARE ids text[];
BEGIN
  -- Negative control: UNION ALL path-set (can explode on hubs)
  EXECUTE format($f$
    WITH RECURSIVE walk AS (
      SELECT $1::text AS node_id, 0 AS depth
      UNION ALL
      SELECT CASE WHEN e.source_id = w.node_id THEN e.target_id ELSE e.source_id END,
             w.depth + 1
      FROM walk w
      JOIN spec164.edges_%s e
        ON e.source_id = w.node_id OR e.target_id = w.node_id
      WHERE w.depth < $2
    )
    SELECT COALESCE(array_agg(node_id ORDER BY node_id), ARRAY[]::text[])
    FROM (
      SELECT DISTINCT node_id FROM walk LIMIT $3
    ) s
  $f$, p_suffix)
  INTO ids
  USING p_start, p_depth, p_max_nodes;
  RETURN ids;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_unroll2(
  p_suffix text,
  p_start text,
  p_max_nodes int
) RETURNS text[]
LANGUAGE plpgsql AS $$
DECLARE ids text[];
BEGIN
  EXECUTE format($f$
    WITH hop1 AS (
      SELECT CASE WHEN e.source_id = $1 THEN e.target_id ELSE e.source_id END AS node_id
      FROM spec164.edges_%s e
      WHERE e.source_id = $1 OR e.target_id = $1
    ),
    hop2 AS (
      SELECT CASE WHEN e.source_id = h.node_id THEN e.target_id ELSE e.source_id END AS node_id
      FROM hop1 h
      JOIN spec164.edges_%s e
        ON e.source_id = h.node_id OR e.target_id = h.node_id
    )
    SELECT COALESCE(array_agg(DISTINCT node_id ORDER BY node_id), ARRAY[]::text[])
    FROM (
      SELECT $1::text AS node_id
      UNION
      SELECT node_id FROM hop1
      UNION
      SELECT node_id FROM hop2
      LIMIT $2
    ) s
  $f$, p_suffix, p_suffix)
  INTO ids
  USING p_start, p_max_nodes;
  RETURN ids;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_upsert_batch(p_suffix text, p_n int, p_e int)
RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  EXECUTE format($f$
    INSERT INTO spec164.nodes_%s (node_id, tenant_id, workspace_id, entity_type, properties)
    SELECT 'u' || g, 't164', 'w164', 'ENTITY', '{}'::jsonb
    FROM generate_series(1, %s) g
    ON CONFLICT (node_id) DO UPDATE SET properties = EXCLUDED.properties
  $f$, p_suffix, p_n);
  EXECUTE format($f$
    INSERT INTO spec164.edges_%s (source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
    SELECT 'u' || ((g - 1) %% %s + 1), 'u' || (g %% %s + 1), 'U', 't164', 'w164', 1.0, '{}'::jsonb
    FROM generate_series(1, %s) g
    ON CONFLICT DO NOTHING
  $f$, p_suffix, p_n, p_n, p_e);
END;
$$;

-- ---------------------------------------------------------------------------
-- Ops: AGE native SQL (plpgsql so CREATE FUNCTION does not require the graph)
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION spec164.age_point_get(p_id text)
RETURNS jsonb
LANGUAGE plpgsql AS $$
DECLARE r jsonb;
BEGIN
  EXECUTE $q$
    SELECT ag_catalog.agtype_to_json(n.properties)::jsonb
    FROM spec164_age."Node" n
    WHERE n.eq_node_id = $1
  $q$ INTO r USING p_id;
  RETURN r;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_incident(p_ids text[])
RETURNS SETOF text
LANGUAGE plpgsql AS $$
BEGIN
  RETURN QUERY EXECUTE $q$
    SELECT e.eq_source_id || '->' || e.eq_target_id
    FROM spec164_age."EDGE" e
    WHERE e.eq_source_id = ANY($1) OR e.eq_target_id = ANY($1)
  $q$ USING p_ids;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_degree_batch(p_ids text[])
RETURNS TABLE(node_id text, deg bigint)
LANGUAGE plpgsql AS $$
BEGIN
  RETURN QUERY EXECUTE $q$
    SELECT u.node_id,
           COUNT(e.*)::bigint
    FROM unnest($1) AS u(node_id)
    LEFT JOIN spec164_age."EDGE" e
      ON e.eq_source_id = u.node_id OR e.eq_target_id = u.node_id
    GROUP BY u.node_id
  $q$ USING p_ids;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_bfs(
  p_start text,
  p_depth int,
  p_max_nodes int
) RETURNS text[]
LANGUAGE plpgsql AS $$
DECLARE
  visited text[] := ARRAY[p_start];
  frontier text[] := ARRAY[p_start];
  nxt text[];
  d int;
BEGIN
  FOR d IN 1..p_depth LOOP
    EXIT WHEN frontier IS NULL OR cardinality(visited) >= p_max_nodes;
    SELECT COALESCE(array_agg(DISTINCT x ORDER BY x), ARRAY[]::text[])
    INTO nxt
    FROM (
      SELECT CASE WHEN e.eq_source_id = ANY(frontier) THEN e.eq_target_id ELSE e.eq_source_id END AS x
      FROM spec164_age."EDGE" e
      WHERE e.eq_source_id = ANY(frontier) OR e.eq_target_id = ANY(frontier)
    ) s
    WHERE x <> ALL(visited);
    nxt := nxt[1:greatest(p_max_nodes - cardinality(visited), 0)];
    visited := visited || nxt;
    frontier := nxt;
  END LOOP;
  RETURN visited[1:p_max_nodes];
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_upsert_batch(p_n int, p_e int)
RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
  PERFORM set_config('search_path', 'ag_catalog, "$user", public', true);
  INSERT INTO spec164_age."Node" (id, properties, eq_node_id)
  SELECT
    public.eq_next_node_id('spec164_age'),
    format(
      '{"node_id": %s, "tenant_id": "t164", "workspace_id": "w164"}',
      to_json('u' || g)::text
    )::ag_catalog.agtype,
    'u' || g
  FROM generate_series(1, p_n) g
  ON CONFLICT (eq_node_id) DO UPDATE
    SET properties = EXCLUDED.properties;

  INSERT INTO spec164_age."EDGE" (id, start_id, end_id, properties, eq_source_id, eq_target_id, eq_rel_type)
  SELECT
    public.eq_next_edge_id('spec164_age'),
    ns.id,
    nt.id,
    format(
      '{"source_id": %s, "target_id": %s, "relation_type": "U", "tenant_id": "t164", "workspace_id": "w164"}',
      to_json(ns.eq_node_id)::text,
      to_json(nt.eq_node_id)::text
    )::ag_catalog.agtype,
    ns.eq_node_id,
    nt.eq_node_id,
    'U'
  FROM generate_series(1, p_e) g
  JOIN spec164_age."Node" ns ON ns.eq_node_id = 'u' || ((g - 1) % p_n + 1)
  JOIN spec164_age."Node" nt ON nt.eq_node_id = 'u' || (g % p_n + 1)
  ON CONFLICT (eq_source_id, eq_target_id, eq_rel_type) DO NOTHING;
END;
$$;

-- ---------------------------------------------------------------------------
-- Timed runner helpers
-- ---------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION spec164.time_ms(p_sql text)
RETURNS double precision
LANGUAGE plpgsql AS $$
DECLARE t0 timestamptz; t1 timestamptz;
BEGIN
  t0 := clock_timestamp();
  EXECUTE p_sql;
  t1 := clock_timestamp();
  RETURN EXTRACT(EPOCH FROM (t1 - t0)) * 1000.0;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.record_run(
  p_scale text, p_arm text, p_op text, p_ix int, p_ms double precision, p_rows bigint DEFAULT NULL, p_notes text DEFAULT NULL
) RETURNS void
LANGUAGE sql AS $$
  INSERT INTO spec164.bench_runs(scale_id, arm_id, op_id, run_ix, elapsed_ms, rows_out, notes)
  VALUES (p_scale, p_arm, p_op, p_ix, p_ms, p_rows, p_notes);
$$;

CREATE OR REPLACE FUNCTION spec164.median_ms(p_scale text, p_arm text, p_op text)
RETURNS double precision
LANGUAGE sql AS $$
  SELECT percentile_cont(0.5) WITHIN GROUP (ORDER BY elapsed_ms)
  FROM spec164.bench_runs
  WHERE scale_id = p_scale AND arm_id = p_arm AND op_id = p_op AND run_ix >= 1;
$$;

-- ===========================================================================
-- Inventory extension ops (OP-164-09..14) — Q-GRAPH-006/008/010/017/020/022
-- ===========================================================================

CREATE OR REPLACE FUNCTION spec164.age_edges_for_node_set(p_ids text[])
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  -- Shipped Q-GRAPH-006 shape: child EDGE + eq_* btree (expand.rs).
  EXECUTE $q$
    SELECT count(*)::bigint
    FROM spec164_age."EDGE" e
    WHERE e.eq_source_id = ANY($1::text[])
      AND e.eq_target_id = ANY($1::text[])
  $q$ INTO n USING p_ids;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_edges_for_node_set_legacy(p_ids text[])
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  -- Pre-rewrite shape, kept so a run can still show the parent-table cost.
  EXECUTE $q$
    SELECT count(*)::bigint
    FROM spec164_age."_ag_label_edge" e
    WHERE ag_catalog.agtype_to_json(e.properties)->>'source_id' = ANY($1::text[])
      AND ag_catalog.agtype_to_json(e.properties)->>'target_id' = ANY($1::text[])
  $q$ INTO n USING p_ids;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_edges_for_node_set(p_suffix text, p_ids text[])
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  EXECUTE format($f$
    SELECT count(*)::bigint
    FROM spec164.edges_%s e
    WHERE e.source_id = ANY($1) AND e.target_id = ANY($1)
  $f$, p_suffix) INTO n USING p_ids;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_search_nodes(p_q text, p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  -- Product shape: LIKE on agtype node_id/description (vertex_filter.rs)
  EXECUTE $q$
    SELECT count(*)::bigint FROM (
      SELECT 1
      FROM spec164_age."Node" v
      WHERE LOWER(ag_catalog.agtype_to_json(v.properties)->>'node_id') LIKE '%' || lower($1) || '%'
         OR LOWER(COALESCE(ag_catalog.agtype_to_json(v.properties)->>'description', '')) LIKE '%' || lower($1) || '%'
      LIMIT $2
    ) s
  $q$ INTO n USING p_q, p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_search_nodes(p_suffix text, p_q text, p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  EXECUTE format($f$
    SELECT count(*)::bigint FROM (
      SELECT 1
      FROM spec164.nodes_%s v
      WHERE lower(v.node_id) LIKE '%%' || lower($1) || '%%'
         OR lower(COALESCE(v.properties->>'description', '')) LIKE '%%' || lower($1) || '%%'
      LIMIT $2
    ) s
  $f$, p_suffix) INTO n USING p_q, p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_popular_nodes(p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  -- Product shape: filter Node, hash-aggregate EDGE.eq_source_id (search.rs).
  EXECUTE $q$
    SELECT count(*)::bigint FROM (
      WITH filtered_nodes AS MATERIALIZED (
        SELECT v.eq_node_id AS node_id
        FROM spec164_age."Node" v
      ),
      edge_counts AS (
        SELECT e.eq_source_id AS node_id, COUNT(*) AS out_degree
        FROM spec164_age."EDGE" e
        INNER JOIN filtered_nodes fn ON e.eq_source_id = fn.node_id
        WHERE e.eq_source_id IS NOT NULL
        GROUP BY 1
      )
      SELECT fn.node_id
      FROM filtered_nodes fn
      LEFT JOIN edge_counts ec ON fn.node_id = ec.node_id
      ORDER BY COALESCE(ec.out_degree, 0) DESC, fn.node_id
      LIMIT $1
    ) s
  $q$ INTO n USING p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_popular_nodes(p_suffix text, p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  -- Pre-degree baseline. Product popular_nodes now reads eq_graph_nodes.degree
  -- (index scan, LIMIT k). This function still hash-aggregates the scratch edges.
  EXECUTE format($f$
    SELECT count(*)::bigint FROM (
      WITH degrees AS MATERIALIZED (
        SELECT node_id, COUNT(*)::bigint AS degree
        FROM (
          SELECT e.source_id AS node_id FROM spec164.edges_%s e
          UNION ALL
          SELECT e.target_id FROM spec164.edges_%s e
        ) ends
        GROUP BY node_id
      )
      SELECT v.node_id
      FROM spec164.nodes_%s v
      LEFT JOIN degrees d ON d.node_id = v.node_id
      ORDER BY COALESCE(d.degree, 0) DESC, v.node_id
      LIMIT $1
    ) s
  $f$, p_suffix, p_suffix, p_suffix) INTO n USING p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_source_prefix_discover(p_exact text[], p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  -- Simplified modern path: jsonb ?| on source_ids (lineage_discovery_sql spirit)
  EXECUTE $q$
    SELECT count(*)::bigint FROM (
      SELECT 1
      FROM spec164_age."Node" v
      WHERE (ag_catalog.agtype_to_json(v.properties)::jsonb->'source_ids') ?| $1::text[]
      LIMIT $2
    ) s
  $q$ INTO n USING p_exact, p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_source_prefix_discover(p_suffix text, p_exact text[], p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  EXECUTE format($f$
    SELECT count(*)::bigint FROM (
      SELECT 1
      FROM spec164.nodes_%s v
      WHERE (v.properties->'source_ids') ?| $1::text[]
      LIMIT $2
    ) s
  $f$, p_suffix) INTO n USING p_exact, p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_list_nodes(p_offset int, p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  EXECUTE $q$
    SELECT count(*)::bigint FROM (
      SELECT 1 FROM spec164_age."Node" v
      WHERE ag_catalog.agtype_to_json(v.properties)->>'tenant_id' = 't164'
        AND ag_catalog.agtype_to_json(v.properties)->>'workspace_id' = 'w164'
      ORDER BY v.eq_node_id
      OFFSET $1 LIMIT $2
    ) s
  $q$ INTO n USING p_offset, p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_list_nodes(p_suffix text, p_offset int, p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  EXECUTE format($f$
    SELECT count(*)::bigint FROM (
      SELECT 1 FROM spec164.nodes_%s v
      WHERE v.tenant_id = 't164' AND v.workspace_id = 'w164'
      ORDER BY v.node_id
      OFFSET $1 LIMIT $2
    ) s
  $f$, p_suffix) INTO n USING p_offset, p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.age_scan_after(p_after text, p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  EXECUTE $q$
    SELECT count(*)::bigint FROM (
      SELECT 1 FROM spec164_age."Node" v
      WHERE v.eq_node_id > $1
      ORDER BY v.eq_node_id
      LIMIT $2
    ) s
  $q$ INTO n USING p_after, p_limit;
  RETURN n;
END;
$$;

CREATE OR REPLACE FUNCTION spec164.plain_scan_after(p_suffix text, p_after text, p_limit int)
RETURNS bigint
LANGUAGE plpgsql AS $$
DECLARE n bigint;
BEGIN
  EXECUTE format($f$
    SELECT count(*)::bigint FROM (
      SELECT 1 FROM spec164.nodes_%s v
      WHERE v.node_id > $1
      ORDER BY v.node_id
      LIMIT $2
    ) s
  $f$, p_suffix) INTO n USING p_after, p_limit;
  RETURN n;
END;
$$;

-- Copy spec164_age into the migration-170 tables and return
-- eq_graph_shadow_mismatches (50 starts, depth 2, cap 500). Requires 170.
-- Static SQL stays inside EXECUTE so this function can be created first.
CREATE OR REPLACE FUNCTION spec164.replay_product_shadow(p_graph text)
RETURNS integer
LANGUAGE plpgsql AS $$
DECLARE
  mismatches integer;
  ns text := p_graph;
BEGIN
  IF p_graph !~ '^[A-Za-z_][A-Za-z0-9_]*$' THEN
    RAISE EXCEPTION 'refusing unsafe graph name';
  END IF;
  IF to_regclass('public.eq_graph_nodes') IS NULL
     OR to_regprocedure('public.eq_graph_shadow_mismatches(text,text)') IS NULL THEN
    RAISE EXCEPTION 'migration 170 is required before shadow replay';
  END IF;

  EXECUTE 'SELECT public.eq_graph_install_mirror_triggers($1, $2)' USING p_graph, ns;
  EXECUTE 'DELETE FROM public.eq_graph_edges WHERE namespace = $1' USING ns;
  EXECUTE 'DELETE FROM public.eq_graph_nodes WHERE namespace = $1' USING ns;

  EXECUTE format($q$
    INSERT INTO public.eq_graph_nodes
      (namespace, node_id, tenant_id, workspace_id, entity_type, properties)
    SELECT $1, d.nid,
           COALESCE(d.props->>'tenant_id', ''),
           COALESCE(d.props->>'workspace_id', ''),
           NULLIF(d.props->>'entity_type', ''),
           d.props
    FROM (
      SELECT DISTINCT ON (nid)
             COALESCE(n.eq_node_id, ag_catalog.agtype_to_json(n.properties)->>'node_id') AS nid,
             ag_catalog.agtype_to_json(n.properties)::jsonb AS props
      FROM %I."Node" n
      WHERE COALESCE(n.eq_node_id, ag_catalog.agtype_to_json(n.properties)->>'node_id') IS NOT NULL
      ORDER BY 1, n.ctid DESC
    ) d
    ON CONFLICT (namespace, node_id) DO UPDATE SET
      tenant_id = EXCLUDED.tenant_id,
      workspace_id = EXCLUDED.workspace_id,
      entity_type = EXCLUDED.entity_type,
      properties = public.eq_merge_graph_properties(
          public.eq_graph_nodes.properties, EXCLUDED.properties)
  $q$, p_graph) USING ns;

  EXECUTE format($q$
    INSERT INTO public.eq_graph_edges
      (namespace, source_id, target_id, rel_type, tenant_id, workspace_id, weight, properties)
    SELECT $1, d.src, d.tgt, d.rel,
           COALESCE(d.props->>'tenant_id', ''),
           COALESCE(d.props->>'workspace_id', ''),
           CASE WHEN COALESCE(d.props->>'weight', '') ~ '^-?[0-9]+(\.[0-9]+)?$'
                THEN (d.props->>'weight')::float8 ELSE NULL END,
           d.props
    FROM (
      SELECT DISTINCT ON (src, tgt, rel)
             COALESCE(e.eq_source_id, ag_catalog.agtype_to_json(e.properties)->>'source_id') AS src,
             COALESCE(e.eq_target_id, ag_catalog.agtype_to_json(e.properties)->>'target_id') AS tgt,
             COALESCE(NULLIF(e.eq_rel_type, ''), NULLIF(ag_catalog.agtype_to_json(e.properties)->>'relation_type', ''), '') AS rel,
             ag_catalog.agtype_to_json(e.properties)::jsonb AS props
      FROM %I."EDGE" e
    ) d
    WHERE EXISTS (
      SELECT 1 FROM public.eq_graph_nodes n WHERE n.namespace = $1 AND n.node_id = d.src
    ) AND EXISTS (
      SELECT 1 FROM public.eq_graph_nodes n WHERE n.namespace = $1 AND n.node_id = d.tgt
    )
    ON CONFLICT (namespace, source_id, target_id, rel_type) DO UPDATE SET
      properties = public.eq_merge_graph_properties(
          public.eq_graph_edges.properties, EXCLUDED.properties),
      tenant_id = EXCLUDED.tenant_id,
      workspace_id = EXCLUDED.workspace_id,
      weight = EXCLUDED.weight
  $q$, p_graph) USING ns;

  PERFORM gin_clean_pending_list('public.idx_eq_graph_nodes_source_ids_gin'::regclass);
  PERFORM gin_clean_pending_list('public.idx_eq_graph_nodes_source_chunk_ids_gin'::regclass);
  PERFORM gin_clean_pending_list('public.idx_eq_graph_edges_source_ids_gin'::regclass);
  PERFORM gin_clean_pending_list('public.idx_eq_graph_edges_source_chunk_ids_gin'::regclass);

  EXECUTE 'SELECT public.eq_graph_shadow_mismatches($1, $2)'
    INTO mismatches USING ns, p_graph;
  RETURN mismatches;
END;
$$;
