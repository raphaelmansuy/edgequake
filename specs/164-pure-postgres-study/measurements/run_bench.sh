#!/usr/bin/env bash
# SPEC-164 measurement driver. Scratch schema only.
set -euo pipefail
# Fail the suite if any timed op errors (psql ON_ERROR_STOP already set in psqlc).

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MEAS="$ROOT/measurements"
EXPLAIN_DIR="$MEAS/explains"
mkdir -p "$EXPLAIN_DIR"

CONTAINER="${SPEC164_PG_CONTAINER:-eqdoc-scratch}"
PGUSER="${SPEC164_PG_USER:-edgequake}"
PGDB="${SPEC164_PG_DB:-edgequake}"
SKIP_L="${SPEC164_SKIP_L:-0}"
M_BUDGET_SEC="${SPEC164_M_BUDGET_SEC:-900}"

psqlc() {
  docker exec -i "$CONTAINER" psql -U "$PGUSER" -d "$PGDB" -v ON_ERROR_STOP=1 "$@"
}

echo "== SPEC-164 bench on container=$CONTAINER db=$PGDB =="
echo "== $(date -u +%Y-%m-%dT%H:%M:%SZ) =="

# Identity
META_JSON="$(psqlc -At -c "SELECT json_build_object(
  'postgres_version', version(),
  'age', (SELECT extversion FROM pg_extension WHERE extname='age'),
  'vector', (SELECT extversion FROM pg_extension WHERE extname='vector'),
  'server_time', clock_timestamp()
);")"

echo "-- load harness.sql"
docker cp "$MEAS/harness.sql" "$CONTAINER:/tmp/spec164_harness.sql"
docker exec "$CONTAINER" psql -U "$PGUSER" -d "$PGDB" -v ON_ERROR_STOP=1 \
  -f /tmp/spec164_harness.sql >/tmp/spec164_harness.log 2>&1 || {
  echo "harness load failed:"; tail -80 /tmp/spec164_harness.log; exit 1
}

run_scale() {
  local scale="$1" n_nodes="$2" n_edges="$3" hub="$4"
  local suffix="$scale"
  echo "== SCALE-$scale nodes=$n_nodes edges=$n_edges hub=$hub =="
  local t0
  t0=$(date +%s)
  psqlc -c "SELECT spec164.load_scale('$suffix', $n_nodes, $n_edges, $hub);" \
    >/tmp/spec164_load_${scale}.log 2>&1 || {
    echo "load failed:"; tail -40 /tmp/spec164_load_${scale}.log; return 1
  }
  local t1
  t1=$(date +%s)
  echo "   load wall_sec=$((t1 - t0))"

  # Counts
  psqlc -At -c "SELECT 'plain_nodes='||count(*) FROM spec164.nodes_${suffix};
                SELECT 'plain_edges='||count(*) FROM spec164.edges_${suffix};
                SELECT 'age_nodes='||count(*) FROM spec164_age.\"Node\";
                SELECT 'age_edges='||count(*) FROM spec164_age.\"EDGE\";"

  # Pick start ids
  local start1 start50 start200
  start1="$(psqlc -At -c "SELECT node_id FROM spec164.nodes_${suffix} ORDER BY md5(node_id||'164') LIMIT 1")"
  mapfile -t IDS50 < <(psqlc -At -c "SELECT node_id FROM spec164.nodes_${suffix} ORDER BY md5(node_id||'50') LIMIT 50")
  mapfile -t IDS200 < <(psqlc -At -c "SELECT node_id FROM spec164.nodes_${suffix} ORDER BY md5(node_id||'200') LIMIT 200")
  local arr50 arr200
  arr50="$(printf "'%s'," "${IDS50[@]}" | sed 's/,$//')"
  arr200="$(printf "'%s'," "${IDS200[@]}" | sed 's/,$//')"

  time_op() {
    local arm="$1" op="$2" sql_stmt="$3"
    psqlc -c "$sql_stmt" >/dev/null
    local i ms
    for i in 0 1 2 3 4 5; do
      ms="$(psqlc -At -c "SELECT spec164.time_ms(\$bench\$${sql_stmt}\$bench\$)")"
      if [[ "$i" -ge 1 ]]; then
        psqlc -c "SELECT spec164.record_run('$scale','$arm','$op',$i,$ms,NULL,NULL);" >/dev/null
      fi
    done
    local med
    med="$(psqlc -At -c "SELECT round(spec164.median_ms('$scale','$arm','$op')::numeric, 3);")"
    echo "   $arm $op median_ms=$med"
  }

  # OP-164-01 point get
  time_op ARM-A OP-164-01 "SELECT spec164.age_point_get('$start1')"
  time_op ARM-B OP-164-01 "SELECT spec164.plain_point_get('$suffix','$start1')"

  # OP-164-02 F=1
  time_op ARM-A OP-164-02 "SELECT count(*) FROM spec164.age_incident(ARRAY['$start1'])"
  time_op ARM-B OP-164-02 "SELECT count(*) FROM spec164.plain_incident('$suffix', ARRAY['$start1'])"

  # OP-164-03 F=50
  time_op ARM-A OP-164-03 "SELECT count(*) FROM spec164.age_incident(ARRAY[$arr50])"
  time_op ARM-B OP-164-03 "SELECT count(*) FROM spec164.plain_incident('$suffix', ARRAY[$arr50])"

  # OP-164-04 F=200
  time_op ARM-A OP-164-04 "SELECT count(*) FROM spec164.age_incident(ARRAY[$arr200])"
  time_op ARM-B OP-164-04 "SELECT count(*) FROM spec164.plain_incident('$suffix', ARRAY[$arr200])"

  # OP-164-05 degree batch
  time_op ARM-A OP-164-05 "SELECT count(*) FROM spec164.age_degree_batch(ARRAY[$arr50])"
  time_op ARM-B OP-164-05 "SELECT count(*) FROM spec164.plain_degree_batch('$suffix', ARRAY[$arr50])"

  # OP-164-06 2-hop
  time_op ARM-A OP-164-06 "SELECT cardinality(spec164.age_bfs('$start1',2,500))"
  time_op ARM-B OP-164-06 "SELECT cardinality(spec164.plain_bfs('$suffix','$start1',2,500))"
  time_op ARM-C-level OP-164-06 "SELECT cardinality(spec164.plain_level_cte('$suffix','$start1',2,500))"
  time_op ARM-C-unroll OP-164-06 "SELECT cardinality(spec164.plain_unroll2('$suffix','$start1',500))"

  # OP-164-07 3-hop
  time_op ARM-A OP-164-07 "SELECT cardinality(spec164.age_bfs('$start1',3,500))"
  time_op ARM-B OP-164-07 "SELECT cardinality(spec164.plain_bfs('$suffix','$start1',3,500))"
  time_op ARM-C-level OP-164-07 "SELECT cardinality(spec164.plain_level_cte('$suffix','$start1',3,500))"

  # Path-set negative control — one session so statement_timeout sticks.
  if [[ "$hub" -gt 0 ]]; then
    set +e
    psqlc <<PATHSQL >/tmp/spec164_path_${scale}.log 2>&1
SET statement_timeout = '120s';
SELECT cardinality(spec164.plain_path_cte('$suffix','n1',3,500)); -- warm
SELECT spec164.record_run('$scale','ARM-C-path','OP-164-07',1,
  spec164.time_ms(\$bench\$SELECT cardinality(spec164.plain_path_cte('$suffix','n1',3,500))\$bench\$), NULL, NULL);
SELECT spec164.record_run('$scale','ARM-C-path','OP-164-07',2,
  spec164.time_ms(\$bench\$SELECT cardinality(spec164.plain_path_cte('$suffix','n1',3,500))\$bench\$), NULL, NULL);
SELECT spec164.record_run('$scale','ARM-C-path','OP-164-07',3,
  spec164.time_ms(\$bench\$SELECT cardinality(spec164.plain_path_cte('$suffix','n1',3,500))\$bench\$), NULL, NULL);
SELECT spec164.record_run('$scale','ARM-C-path','OP-164-07',4,
  spec164.time_ms(\$bench\$SELECT cardinality(spec164.plain_path_cte('$suffix','n1',3,500))\$bench\$), NULL, NULL);
SELECT spec164.record_run('$scale','ARM-C-path','OP-164-07',5,
  spec164.time_ms(\$bench\$SELECT cardinality(spec164.plain_path_cte('$suffix','n1',3,500))\$bench\$), NULL, NULL);
SET statement_timeout = 0;
PATHSQL
    local path_rc=$?
    set -e
    if [[ "$path_rc" -ne 0 ]]; then
      psqlc -c "SELECT spec164.record_run('$scale','ARM-C-path','OP-164-07',1,120000,NULL,'timeout_or_error');" >/dev/null
      echo "   ARM-C-path OP-164-07 timeout_or_error (recorded 120000)"
      tail -20 "/tmp/spec164_path_${scale}.log" || true
    else
      local med
      med="$(psqlc -At -c "SELECT round(spec164.median_ms('$scale','ARM-C-path','OP-164-07')::numeric, 3);")"
      echo "   ARM-C-path OP-164-07 median_ms=$med"
    fi
  fi

  # OP-164-08 upsert batch (200 nodes + 500 edges)
  time_op ARM-A OP-164-08 "SELECT spec164.age_upsert_batch(200,500)"
  time_op ARM-B OP-164-08 "SELECT spec164.plain_upsert_batch('$suffix',200,500)"

  # EXPLAIN samples
  psqlc -c "EXPLAIN (ANALYZE, BUFFERS) SELECT * FROM spec164.age_incident(ARRAY['$start1']);" \
    >"$EXPLAIN_DIR/${scale}_ARM-A_incident.txt" 2>&1 || true
  psqlc -c "EXPLAIN (ANALYZE, BUFFERS) SELECT * FROM spec164.plain_incident('$suffix', ARRAY['$start1']);" \
    >"$EXPLAIN_DIR/${scale}_ARM-B_incident.txt" 2>&1 || true
  psqlc -c "EXPLAIN (ANALYZE, BUFFERS) SELECT cardinality(spec164.plain_level_cte('$suffix','$start1',2,500));" \
    >"$EXPLAIN_DIR/${scale}_ARM-C-level_d2.txt" 2>&1 || true

  # Correctness: 50 starts, depth 2
  echo "   correctness depth=2 n=50"
  psqlc <<SQL
DELETE FROM spec164.correctness WHERE scale_id = '$scale';
-- Hash sorted id sets (order differs between BFS and CTE; LAW-164-02 cares about membership).
INSERT INTO spec164.correctness (scale_id, start_id, arm_a_hash, arm_b_hash, arm_c_hash, ok)
SELECT
  '$scale',
  s.node_id,
  md5(array_to_string(ARRAY(SELECT unnest(spec164.age_bfs(s.node_id, 2, 500)) ORDER BY 1), ',')),
  md5(array_to_string(ARRAY(SELECT unnest(spec164.plain_bfs('$suffix', s.node_id, 2, 500)) ORDER BY 1), ',')),
  md5(array_to_string(ARRAY(SELECT unnest(spec164.plain_level_cte('$suffix', s.node_id, 2, 500)) ORDER BY 1), ',')),
  md5(array_to_string(ARRAY(SELECT unnest(spec164.age_bfs(s.node_id, 2, 500)) ORDER BY 1), ','))
    = md5(array_to_string(ARRAY(SELECT unnest(spec164.plain_bfs('$suffix', s.node_id, 2, 500)) ORDER BY 1), ','))
  AND md5(array_to_string(ARRAY(SELECT unnest(spec164.plain_bfs('$suffix', s.node_id, 2, 500)) ORDER BY 1), ','))
    = md5(array_to_string(ARRAY(SELECT unnest(spec164.plain_level_cte('$suffix', s.node_id, 2, 500)) ORDER BY 1), ','))
FROM (
  SELECT node_id FROM spec164.nodes_${suffix}
  ORDER BY md5(node_id || 'corr')
  LIMIT 50
) s;
SQL
  local okc
  okc="$(psqlc -At -c "SELECT count(*) FILTER (WHERE ok), count(*) FROM spec164.correctness WHERE scale_id='$scale'")"
  echo "   correctness ok/total=$okc"

  echo "$((t1 - t0))" >"/tmp/spec164_loadsec_${scale}.txt"
}

# Clear prior runs
psqlc -c "TRUNCATE spec164.bench_runs; TRUNCATE spec164.correctness;" >/dev/null 2>&1 || true

run_scale s 50000 70000 0
M_START=$(date +%s)
run_scale m 200000 500000 0
M_END=$(date +%s)
M_ELAPSED=$((M_END - M_START))
echo "SCALE-M suite wall_sec=$M_ELAPSED budget=$M_BUDGET_SEC"

run_scale hub 200000 500000 1000

RAN_L=false
if [[ "$SKIP_L" != "1" && "$M_ELAPSED" -lt "$M_BUDGET_SEC" ]]; then
  echo "SCALE-M under budget; running SCALE-L"
  run_scale l 1000000 2000000 0 || echo "SCALE-L failed or aborted"
  RAN_L=true
else
  echo "Skipping SCALE-L (SKIP_L=$SKIP_L or M wall $M_ELAPSED >= $M_BUDGET_SEC)"
fi

# Assemble results.json
python3 - "$MEAS/results.json" "$META_JSON" "$RAN_L" "$M_ELAPSED" <<'PY'
import json, sys, subprocess, os

out_path, meta_raw, ran_l, m_elapsed = sys.argv[1], sys.argv[2], sys.argv[3] == "true", int(sys.argv[4])
container = os.environ.get("SPEC164_PG_CONTAINER", "eqdoc-scratch")
user = os.environ.get("SPEC164_PG_USER", "edgequake")
db = os.environ.get("SPEC164_PG_DB", "edgequake")

def q(sql):
    r = subprocess.check_output(
        ["docker", "exec", "-i", container, "psql", "-U", user, "-d", db, "-At", "-c", sql],
        text=True,
    ).strip()
    return r

rows = q("""
SELECT json_agg(row_to_json(t)) FROM (
  SELECT scale_id, arm_id, op_id,
         round(percentile_cont(0.5) WITHIN GROUP (ORDER BY elapsed_ms)::numeric, 3) AS median_ms,
         round(min(elapsed_ms)::numeric, 3) AS min_ms,
         round(max(elapsed_ms)::numeric, 3) AS max_ms,
         count(*) AS n
  FROM spec164.bench_runs
  WHERE run_ix >= 1
  GROUP BY 1,2,3
  ORDER BY 1,3,2
) t;
""")
medians = json.loads(rows) if rows else []

corr = q("""
SELECT json_agg(row_to_json(t)) FROM (
  SELECT scale_id,
         count(*) FILTER (WHERE ok) AS ok,
         count(*) AS total
  FROM spec164.correctness
  GROUP BY 1
) t;
""")
correctness = json.loads(corr) if corr else []

# ratios B/A
by = {}
for r in medians:
    by[(r["scale_id"], r["op_id"], r["arm_id"])] = float(r["median_ms"])

ratios = []
for scale, op in sorted({(r["scale_id"], r["op_id"]) for r in medians}):
    a = by.get((scale, op, "ARM-A"))
    b = by.get((scale, op, "ARM-B"))
    if a and b and a > 0:
        ratios.append({
            "scale_id": scale,
            "op_id": op,
            "ratio_b_over_a": round(b / a, 3),
            "arm_a_ms": a,
            "arm_b_ms": b,
            "within_2x": (b / a) <= 2.0,
        })

# hypotheses evaluation on SCALE-M
m_ratios = [r for r in ratios if r["scale_id"] == "m"]
h01 = all(r["within_2x"] for r in m_ratios) if m_ratios else False
corr_m = next((c for c in correctness if c["scale_id"] == "m"), None)
h02 = corr_m is not None and int(corr_m["ok"]) == int(corr_m["total"]) and int(corr_m["total"]) == 50
# H-164-03: C-level within 2x of B on OP-164-06 at m
b06 = by.get(("m", "OP-164-06", "ARM-B"))
c06 = by.get(("m", "OP-164-06", "ARM-C-level"))
h03 = bool(b06 and c06 and c06 / b06 <= 2.0)
# H-164-04: path > 10x level on hub OP-164-07
c_path = by.get(("hub", "OP-164-07", "ARM-C-path"))
c_level = by.get(("hub", "OP-164-07", "ARM-C-level"))
h04 = bool(c_path and c_level and c_level > 0 and c_path / c_level > 10.0)
# H-164-05 upsert
r08 = next((r for r in m_ratios if r["op_id"] == "OP-164-08"), None)
h05 = bool(r08 and r08["within_2x"])

if h01 and h02 and h05:
    decision = "migrate_feasible"
elif h01 and h02 and not h05:
    decision = "migrate_reads_only"
else:
    decision = "keep_age"

doc = {
    "spec": "164",
    "meta": json.loads(meta_raw),
    "ran_scale_l": ran_l,
    "scale_m_suite_wall_sec": m_elapsed,
    "medians": medians,
    "ratios_b_over_a": ratios,
    "correctness": correctness,
    "hypotheses": {
        "H-164-01": {"pass": h01, "detail": "ARM-B <= 2x ARM-A all ops at SCALE-M"},
        "H-164-02": {"pass": h02, "detail": "50/50 neighbor set equality at SCALE-M"},
        "H-164-03": {"pass": h03, "detail": "level CTE <= 2x iterative B on OP-164-06 SCALE-M"},
        "H-164-04": {"pass": h04, "detail": "path CTE > 10x level on HUB OP-164-07"},
        "H-164-05": {"pass": h05, "detail": "upsert ARM-B <= 2x ARM-A at SCALE-M"},
    },
    "decision_rule_outcome": decision,
}
with open(out_path, "w") as f:
    json.dump(doc, f, indent=2)
    f.write("\n")
print(json.dumps({"wrote": out_path, "decision_rule_outcome": decision}, indent=2))
PY

echo "== done: $MEAS/results.json =="
