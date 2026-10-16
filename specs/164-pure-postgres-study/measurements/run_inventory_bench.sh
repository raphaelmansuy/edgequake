#!/usr/bin/env bash
# SPEC-164 inventory extension: OP-164-09..14 at SCALE-M only.
#
# Transport: local psql when DATABASE_URL is set or SPEC164_PG_MODE=local.
# Docker exec remains the default when neither is set (container eqdoc-scratch).
#
# Writes measurements/inventory_results_shipped.json. The study file
# inventory_results.json is the historical parent-table run and is not replaced.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MEAS="$ROOT/measurements"
REPO="$(cd "$ROOT/../.." && pwd)"
CONTAINER="${SPEC164_PG_CONTAINER:-eqdoc-scratch}"
PGUSER="${SPEC164_PG_USER:-edgequake}"
PGDB="${SPEC164_PG_DB:-edgequake}"
MODE="${SPEC164_PG_MODE:-auto}"

if [[ "$MODE" == "auto" && -n "${DATABASE_URL:-}" ]]; then
  MODE=local
elif [[ "$MODE" == "auto" ]]; then
  MODE=docker
fi

psqlc() {
  if [[ "$MODE" == "local" ]]; then
    if [[ -n "${DATABASE_URL:-}" ]]; then
      psql "$DATABASE_URL" -v ON_ERROR_STOP=1 "$@"
    else
      psql -h "${PGHOST:-127.0.0.1}" -p "${PGPORT:-5432}" -U "$PGUSER" -d "$PGDB" -v ON_ERROR_STOP=1 "$@"
    fi
  else
    docker exec -i "$CONTAINER" psql -U "$PGUSER" -d "$PGDB" -v ON_ERROR_STOP=1 "$@"
  fi
}

apply_sql() {
  local file="$1"
  if [[ "$MODE" == "local" ]]; then
    psqlc -f "$file"
  else
    docker cp "$file" "$CONTAINER:/tmp/$(basename "$file")"
    docker exec "$CONTAINER" psql -U "$PGUSER" -d "$PGDB" -v ON_ERROR_STOP=1 \
      -f "/tmp/$(basename "$file")"
  fi
}

echo "== SPEC-164 inventory bench SCALE-M ($MODE) =="
psqlc -c "CREATE EXTENSION IF NOT EXISTS age CASCADE;"
apply_sql "$REPO/edgequake/migrations/067_native_graph_write_helpers.sql" >/tmp/spec164_inv_067.log
apply_sql "$REPO/edgequake/migrations/075_fix_native_graph_write_helpers.sql" >/tmp/spec164_inv_075.log
apply_sql "$REPO/edgequake/migrations/076_fix_graphid_cast.sql" >/tmp/spec164_inv_076.log
apply_sql "$MEAS/harness.sql" >/tmp/spec164_inv_harness.log

psqlc -c "DELETE FROM spec164.bench_runs WHERE scale_id='m_inv' OR op_id LIKE 'OP-164-1%';" || true
echo "-- load SCALE-M enriched"
psqlc -c "SELECT spec164.load_scale('m', 200000, 500000, 0);"

suffix=m
scale=m_inv

mapfile -t IDS50 < <(psqlc -At -c "SELECT node_id FROM spec164.nodes_m ORDER BY md5(node_id||'inv50') LIMIT 50")
arr50="$(printf "'%s'," "${IDS50[@]}" | sed 's/,$//')"
start1="$(psqlc -At -c "SELECT node_id FROM spec164.nodes_m ORDER BY md5(node_id||'inv') LIMIT 1")"
set_ids="$(psqlc -At -c "SELECT array_to_string(spec164.plain_bfs('m','$start1',2,100), ''',''')")"
set_arr="'$set_ids'"
probes="'doc:0','doc:0_0','doc:0_1','doc:1','doc:1_0'"
echo "start=$start1"

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

time_op ARM-A OP-164-09 "SELECT spec164.age_edges_for_node_set(ARRAY[$set_arr])"
time_op ARM-B OP-164-09 "SELECT spec164.plain_edges_for_node_set('$suffix', ARRAY[$set_arr])"
time_op ARM-A OP-164-09-legacy "SELECT spec164.age_edges_for_node_set_legacy(ARRAY[$set_arr])"
time_op ARM-A OP-164-10 "SELECT spec164.age_search_nodes('n12', 50)"
time_op ARM-B OP-164-10 "SELECT spec164.plain_search_nodes('$suffix', 'n12', 50)"
time_op ARM-A OP-164-11 "SELECT spec164.age_popular_nodes(50)"
time_op ARM-B OP-164-11 "SELECT spec164.plain_popular_nodes('$suffix', 50)"
time_op ARM-A OP-164-12 "SELECT spec164.age_source_prefix_discover(ARRAY[$probes], 5000)"
time_op ARM-B OP-164-12 "SELECT spec164.plain_source_prefix_discover('$suffix', ARRAY[$probes], 5000)"
time_op ARM-A OP-164-13 "SELECT spec164.age_list_nodes(0, 50)"
time_op ARM-B OP-164-13 "SELECT spec164.plain_list_nodes('$suffix', 0, 50)"
time_op ARM-A OP-164-14 "SELECT spec164.age_scan_after('n100000', 500)"
time_op ARM-B OP-164-14 "SELECT spec164.plain_scan_after('$suffix', 'n100000', 500)"

mkdir -p "$MEAS/explains"
psqlc -c "EXPLAIN (ANALYZE, BUFFERS) SELECT spec164.age_edges_for_node_set(ARRAY[$set_arr]);" \
  >"$MEAS/explains/m_inv_shipped_ARM-A_edges_set.txt" 2>&1 || true
psqlc -c "EXPLAIN (ANALYZE, BUFFERS) SELECT spec164.plain_edges_for_node_set('m', ARRAY[$set_arr]);" \
  >"$MEAS/explains/m_inv_shipped_ARM-B_edges_set.txt" 2>&1 || true
psqlc -c "EXPLAIN (ANALYZE, BUFFERS) SELECT spec164.age_source_prefix_discover(ARRAY[$probes], 5000);" \
  >"$MEAS/explains/m_inv_shipped_ARM-A_lineage.txt" 2>&1 || true
psqlc -c "EXPLAIN (ANALYZE, BUFFERS) SELECT spec164.plain_source_prefix_discover('m', ARRAY[$probes], 5000);" \
  >"$MEAS/explains/m_inv_shipped_ARM-B_lineage.txt" 2>&1 || true

echo "-- migration 170 + shadow replay on the SCALE-M graph"
apply_sql "$REPO/edgequake/migrations/090_eq_merge_graph_properties.sql" >/tmp/spec164_inv_090.log
apply_sql "$REPO/edgequake/migrations/170_spec164_plain_graph.sql" >/tmp/spec164_inv_170.log
shadow="$(psqlc -At -c "SELECT spec164.replay_product_shadow('spec164_age');")"
echo "shadow_mismatches=$shadow"
if [[ "$shadow" != "0" ]]; then
  echo "shadow mismatches=$shadow" >&2
  exit 1
fi

rows="$(psqlc -At -c "
SELECT COALESCE(json_agg(row_to_json(t))::text, '[]') FROM (
  SELECT scale_id, arm_id, op_id,
         round(percentile_cont(0.5) WITHIN GROUP (ORDER BY elapsed_ms)::numeric, 3) AS median_ms,
         count(*) AS n
  FROM spec164.bench_runs
  WHERE scale_id = 'm_inv' AND run_ix >= 1
  GROUP BY 1,2,3 ORDER BY 3,2
) t;")"
legacy="$(psqlc -At -c "SELECT round(spec164.median_ms('m_inv','ARM-A','OP-164-09-legacy')::numeric, 3);")"

OUT="$MEAS/inventory_results_shipped.json"
SPEC164_ROWS="$rows" python3 - "$OUT" "$shadow" "$legacy" <<'PY'
import json, os, sys
out, shadow, legacy = sys.argv[1], sys.argv[2], sys.argv[3]
rows = os.environ.get("SPEC164_ROWS", "")
medians = json.loads(rows) if rows.strip() else []
by = {(r["op_id"], r["arm_id"]): float(r["median_ms"]) for r in medians}
ratios = []
all_ok = True
for op in [f"OP-164-{i:02d}" for i in range(9, 15)]:
    a, b = by.get((op, "ARM-A")), by.get((op, "ARM-B"))
    if a and b and a > 0:
        ratio = round(b / a, 3)
        ok = ratio <= 2.0
        if not ok:
            all_ok = False
        ratios.append({"op_id": op, "arm_a_ms": a, "arm_b_ms": b, "ratio_b_over_a": ratio, "within_2x": ok})
    else:
        all_ok = False
        ratios.append({"op_id": op, "error": "missing", "within_2x": False})

r09 = next((r for r in ratios if r.get("op_id") == "OP-164-09"), {})
try:
    legacy_ms = float(legacy)
except ValueError:
    legacy_ms = None
h07 = bool(r09.get("arm_b_ms") and legacy_ms and r09["arm_b_ms"] < legacy_ms)
r12 = next((r for r in ratios if r.get("op_id") == "OP-164-12"), {})
h08 = bool(r12.get("within_2x"))
mismatches = int(shadow)
doc = {
    "spec": "164-inventory-shipped",
    "scale": "m_inv (200k/500k)",
    "age_close": "EDGE.eq_source_id / eq_target_id",
    "shadow_mismatches": mismatches,
    "legacy_parent_close_ms": legacy_ms,
    "medians": medians,
    "ratios_b_over_a": ratios,
    "hypotheses": {
        "H-164-06": {"pass": all_ok and mismatches == 0, "detail": "OP-164-09..14 B<=2x A at M, shipped eq_* close"},
        "H-164-07": {"pass": h07, "detail": "plain close faster than the parent _ag_label_edge query"},
        "H-164-08": {"pass": h08, "detail": "lineage GIN plain within 2x AGE"},
    },
    "decision_amendment": "strengthen_migrate_feasible" if all_ok and mismatches == 0 else "migrate_with_caveat",
}
with open(out, "w") as f:
    json.dump(doc, f, indent=2)
    f.write("\n")
print(json.dumps(doc, indent=2))
if not all_ok or mismatches != 0:
    sys.exit(1)
PY

echo "== wrote $OUT =="
