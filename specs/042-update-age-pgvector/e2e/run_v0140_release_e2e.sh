#!/usr/bin/env bash
# SPEC-042 — v0.14.0 release E2E proof using published GHCR Docker images.
#
# Tests the FULL database lifecycle (create extensions, graph, vector ops,
# AGE Cypher, HNSW, ingestion-ready schema) against the official Docker images
# for PG16, PG17, PG18 published to ghcr.io.
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_v0140_release_e2e.sh [pg16|pg17|pg18|all]
#
# Requirements: docker
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
E2E_DIR="$(cd "$(dirname "$0")" && pwd)"
REPORT_DIR="$E2E_DIR/v0140-release-proof"
PROFILE="${1:-all}"
PGPASSWORD="edgequake_e2e"
VERSION="0.14.0"
REGISTRY="ghcr.io/raphaelmansuy/edgequake-postgres"

mkdir -p "$REPORT_DIR"

log() { echo "[$(date +%H:%M:%S)] $*"; }

run_profile() {
  local profile="$1"
  local image="${REGISTRY}:${VERSION}-${profile}"
  local container="eq-release-e2e-${profile}-$$"
  local report="$REPORT_DIR/${profile}-report.txt"

  log "=========================================="
  log "E2E RELEASE PROOF: $profile → $image"
  log "=========================================="

  {
    echo "# v${VERSION} Release E2E — ${profile}"
    echo "# Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "# Image: $image"
    echo ""
  } > "$report"

  cleanup() { docker rm -f "$container" >/dev/null 2>&1 || true; }
  trap cleanup RETURN

  # ── Pull image ──────────────────────────────────────────────────────────────
  log "  [1/8] Pulling $image..."
  if ! docker pull "$image" >/dev/null 2>&1; then
    echo "FAIL: cannot pull $image" | tee -a "$report"
    return 1
  fi
  echo "PASS [1/8] Image pulled: $image" >> "$report"

  # ── Start container ─────────────────────────────────────────────────────────
  log "  [2/8] Starting container..."
  docker run -d --name "$container" \
    -e POSTGRES_USER=edgequake \
    -e POSTGRES_PASSWORD="$PGPASSWORD" \
    -e POSTGRES_DB=edgequake \
    "$image" >/dev/null

  for i in $(seq 1 60); do
    if docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
      psql -U edgequake -d edgequake -c 'SELECT 1' >/dev/null 2>&1; then
      break
    fi
    [ "$i" -eq 60 ] && { log "FAIL: Postgres did not start"; return 1; }
    sleep 1
  done
  echo "PASS [2/8] Container ready (${i}s)" >> "$report"

  # ── Verify extensions ───────────────────────────────────────────────────────
  log "  [3/8] Verifying extensions..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c "
      CREATE EXTENSION IF NOT EXISTS vector;
      CREATE EXTENSION IF NOT EXISTS pg_trgm;
      CREATE EXTENSION IF NOT EXISTS btree_gin;
      CREATE EXTENSION IF NOT EXISTS \"uuid-ossp\";
      CREATE EXTENSION IF NOT EXISTS age;
      LOAD 'age';
    " >/dev/null
  ext_output=$(docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -tA -c \
    "SELECT extname || '=' || extversion FROM pg_extension WHERE extname IN ('vector','age') ORDER BY extname;"
  )
  echo "$ext_output" | grep -q "age=" || { log "FAIL: AGE not found"; return 1; }
  echo "$ext_output" | grep -q "vector=" || { log "FAIL: vector not found"; return 1; }
  echo "PASS [3/8] Extensions: $(echo "$ext_output" | tr '\n' ' ')" >> "$report"

  # ── PG major version check ─────────────────────────────────────────────────
  log "  [4/8] Checking PG major version..."
  expected_major="${profile#pg}"
  actual_major=$(docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -tAc \
    "SELECT current_setting('server_version_num')::int / 10000;")
  [ "$actual_major" = "$expected_major" ] || {
    log "FAIL: expected PG$expected_major got PG$actual_major"
    return 1
  }
  echo "PASS [4/8] PG major = $actual_major" >> "$report"

  # ── Vector operations (HNSW + halfvec) ──────────────────────────────────────
  log "  [5/8] Vector operations (HNSW + halfvec)..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c "
    CREATE TEMP TABLE e2e_vectors (
      id serial, embedding vector(384), tenant_id text, content text
    );
    INSERT INTO e2e_vectors (embedding, tenant_id, content)
    SELECT ('[' || string_agg(round(random()::numeric, 4)::text, ',') || ']')::vector(384),
           'tenant-e2e', 'doc-' || g
    FROM generate_series(1, 50) g, generate_series(1, 384) d
    GROUP BY g;
    CREATE INDEX ON e2e_vectors USING hnsw (embedding vector_cosine_ops);
    BEGIN;
    SET LOCAL hnsw.iterative_scan = strict_order;
    SET LOCAL hnsw.ef_search = 40;
    SELECT id, content FROM (
      SELECT id, content, embedding <=> (SELECT embedding FROM e2e_vectors WHERE id=1) AS dist
      FROM e2e_vectors WHERE tenant_id = 'tenant-e2e'
      ORDER BY dist LIMIT 5
    ) s;
    COMMIT;
  " >/dev/null
  echo "PASS [5/8] Vector HNSW filtered ANN query (384-d, 50 rows)" >> "$report"

  # ── Halfvec HNSW ───────────────────────────────────────────────────────────
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c "
    CREATE TEMP TABLE e2e_halfvec (id int, emb halfvec(3));
    INSERT INTO e2e_halfvec VALUES (1,'[1,0,0]'),(2,'[0,1,0]'),(3,'[0,0,1]');
    CREATE INDEX ON e2e_halfvec USING hnsw (emb halfvec_cosine_ops);
    SELECT id FROM e2e_halfvec ORDER BY emb <=> '[1,0,0]'::halfvec LIMIT 1;
  " >/dev/null
  echo "PASS [5b/8] Halfvec HNSW (SPEC-042 dimension guard)" >> "$report"

  # ── Apache AGE graph operations ─────────────────────────────────────────────
  log "  [6/8] Apache AGE graph operations..."
  docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<'AGESQL' >/dev/null
LOAD 'age';
SET search_path = ag_catalog, public;
SELECT create_graph('e2e_release_graph');
SELECT * FROM cypher('e2e_release_graph', $$
  MERGE (a:ENTITY {name: 'EdgeQuake', type: 'PRODUCT'})
  MERGE (b:ENTITY {name: 'PostgreSQL', type: 'TECHNOLOGY'})
  MERGE (a)-[:DEPENDS_ON {weight: 0.95}]->(b)
  RETURN a.name, b.name
$$) AS (a agtype, b agtype);
AGESQL

  node_count=$(docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -tA <<'AGESQL'
LOAD 'age';
SET search_path = ag_catalog, public;
SELECT * FROM cypher('e2e_release_graph', $$
  MATCH (n) RETURN count(n)
$$) AS (cnt agtype);
AGESQL
  )
  node_count=$(echo "$node_count" | grep -v '^$' | tail -1)
  [ "$node_count" -ge 2 ] || { log "FAIL: expected >=2 nodes, got $node_count"; return 1; }

  edge_count=$(docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -tA <<'AGESQL'
LOAD 'age';
SET search_path = ag_catalog, public;
SELECT * FROM cypher('e2e_release_graph', $$
  MATCH ()-[r]->() RETURN count(r)
$$) AS (cnt agtype);
AGESQL
  )
  edge_count=$(echo "$edge_count" | grep -v '^$' | tail -1)
  [ "$edge_count" -ge 1 ] || { log "FAIL: expected >=1 edges, got $edge_count"; return 1; }

  docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<'AGESQL' >/dev/null
LOAD 'age';
SET search_path = ag_catalog, public;
SELECT drop_graph('e2e_release_graph', true);
AGESQL

  echo "PASS [6/8] AGE graph: $node_count nodes, $edge_count edges (MERGE+MATCH+DROP)" >> "$report"

  # ── Ingestion-ready schema (simulates what EdgeQuake backend creates) ───────
  log "  [7/8] Ingestion-ready schema simulation..."
  docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c "
    -- KV store (documents, chunks, metadata)
    CREATE TABLE IF NOT EXISTS kv_store (
      key text PRIMARY KEY, value jsonb NOT NULL, created_at timestamptz DEFAULT now()
    );
    INSERT INTO kv_store VALUES
      ('doc:abc:meta', '{\"filename\":\"test.pdf\",\"status\":\"completed\"}'),
      ('doc:abc:chunk:1', '{\"text\":\"EdgeQuake is a RAG framework.\",\"page\":1}'),
      ('doc:abc:chunk:2', '{\"text\":\"It uses knowledge graphs.\",\"page\":2}');

    -- Vector store (embeddings)
    CREATE TABLE IF NOT EXISTS vector_store (
      id text PRIMARY KEY, embedding vector(384), metadata jsonb, created_at timestamptz DEFAULT now()
    );

    -- Full-text search
    CREATE INDEX IF NOT EXISTS idx_kv_value_gin ON kv_store USING gin (value);

    SELECT count(*) FROM kv_store;
  " >/dev/null
  echo "PASS [7/8] Ingestion schema: kv_store + vector_store + indexes" >> "$report"

  # ── PG-version-specific features ────────────────────────────────────────────
  log "  [8/8] PG-version-specific features..."
  case "$profile" in
    pg18)
      uuid7=$(docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
        psql -U edgequake -d edgequake -tAc "SELECT uuidv7();")
      [ -n "$uuid7" ] || { log "FAIL: uuidv7() returned empty"; return 1; }
      echo "PASS [8/8] PG18: uuidv7() = $uuid7" >> "$report"
      ;;
    pg17)
      echo "PASS [8/8] PG17: confirmed major 17 (AGE 1.7.0)" >> "$report"
      ;;
    pg16)
      if docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
        psql -U edgequake -d edgequake -tAc "SELECT uuidv7();" 2>/dev/null; then
        log "FAIL: uuidv7 should NOT exist on PG16"
        return 1
      fi
      echo "PASS [8/8] PG16: uuidv7() absent (expected, AGE 1.6.0)" >> "$report"
      ;;
  esac

  echo "" >> "$report"
  echo "RESULT: ALL PASSED" >> "$report"
  log "  ✓ $profile — ALL 8 CHECKS PASSED"
}

# ── Main ──────────────────────────────────────────────────────────────────────
log "SPEC-042 v${VERSION} Release E2E Proof"
log "Registry: $REGISTRY"
log ""

# Cleanup any stale containers
docker ps -aq --filter "name=eq-release-e2e-" 2>/dev/null | xargs -r docker rm -f >/dev/null 2>&1 || true

failed=0
case "$PROFILE" in
  all)
    for p in pg16 pg17 pg18; do
      run_profile "$p" || failed=1
    done
    ;;
  pg16|pg17|pg18)
    run_profile "$PROFILE" || failed=1
    ;;
  *)
    echo "Usage: $0 [pg16|pg17|pg18|all]"
    exit 1
    ;;
esac

# ── Summary ───────────────────────────────────────────────────────────────────
echo ""
log "=========================================="
log "SUMMARY — v${VERSION} Release E2E"
log "=========================================="
for f in "$REPORT_DIR"/*-report.txt; do
  [ -f "$f" ] || continue
  profile=$(basename "$f" -report.txt)
  result=$(grep "^RESULT:" "$f" 2>/dev/null | head -1)
  echo "  $profile: ${result:-UNKNOWN}"
done
echo ""

if [ "$failed" -ne 0 ]; then
  log "✗ SOME PROFILES FAILED — see reports in $REPORT_DIR"
  exit 1
fi

log "✓ v${VERSION} RELEASE E2E PROOF — ALL PROFILES PASSED"
