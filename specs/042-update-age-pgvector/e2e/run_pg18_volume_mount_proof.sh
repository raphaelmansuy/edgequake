#!/usr/bin/env bash
# SPEC-042/017 — PG18 volume mount fix E2E proof (#280).
#
# Verifies that mounting at /var/lib/postgresql works for PG16, PG17, PG18:
#   1. Fresh start — container boots, extensions load
#   2. Write data — insert rows into a test table
#   3. Restart — stop + start, data persists
#   4. Clean up
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_pg18_volume_mount_proof.sh [pg16|pg17|pg18|all]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
REPORT_DIR="$ROOT/specs/042-update-age-pgvector/e2e/v0140-volume-proof"
PROFILE="${1:-all}"
PGPASSWORD="vol_proof_secret"
VERSION="0.14.0"
REGISTRY="ghcr.io/raphaelmansuy/edgequake-postgres"

mkdir -p "$REPORT_DIR"
log() { echo "[$(date +%H:%M:%S)] $*"; }

run_profile() {
  local profile="$1"
  local image="${REGISTRY}:${VERSION}-${profile}"
  local container="eq-vol-proof-${profile}-$$"
  local volume="eq-vol-proof-${profile}-$$"
  local report="$REPORT_DIR/${profile}-volume-report.txt"
  local mount_path
  case "$profile" in
    pg18) mount_path="/var/lib/postgresql" ;;
    *)    mount_path="/var/lib/postgresql/data" ;;
  esac

  log "=========================================="
  log "VOLUME PROOF: $profile → mount at $mount_path"
  log "=========================================="

  {
    echo "# v${VERSION} Volume Mount Proof — ${profile}"
    echo "# Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "# Image: $image"
    echo "# Mount: $mount_path"
    echo ""
  } > "$report"

  cleanup() {
    docker rm -f "$container" >/dev/null 2>&1 || true
    docker volume rm "$volume" >/dev/null 2>&1 || true
  }
  trap cleanup RETURN

  docker volume create "$volume" >/dev/null

  # ── Step 1: Fresh start ─────────────────────────────────────────────────────
  log "  [1/4] Fresh start with volume mount at $mount_path..."
  docker run -d --name "$container" \
    -v "${volume}:${mount_path}" \
    -e POSTGRES_USER=edgequake \
    -e POSTGRES_PASSWORD="$PGPASSWORD" \
    -e POSTGRES_DB=edgequake \
    "$image" >/dev/null

  for i in $(seq 1 60); do
    if docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
      psql -U edgequake -d edgequake -c 'SELECT 1' >/dev/null 2>&1; then
      break
    fi
    if [ "$i" -eq 60 ]; then
      log "FAIL: container did not start"
      docker logs "$container" 2>&1 | tail -20
      echo "FAIL [1/4] Container did not start" >> "$report"
      return 1
    fi
    sleep 1
  done
  echo "PASS [1/4] Fresh start OK (${i}s)" >> "$report"

  # ── Step 2: Create extensions + write data ──────────────────────────────────
  log "  [2/4] Create extensions + write test data..."
  docker exec -i -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 <<'SQL'
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS age;
CREATE TABLE IF NOT EXISTS persistence_test (
  id serial PRIMARY KEY,
  payload text NOT NULL,
  created_at timestamptz DEFAULT now()
);
INSERT INTO persistence_test (payload) VALUES
  ('row-1-proof-of-persistence'),
  ('row-2-proof-of-persistence'),
  ('row-3-proof-of-persistence');
SQL
  row_count=$(docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -tAc "SELECT count(*) FROM persistence_test;")
  [ "$row_count" = "3" ] || { log "FAIL: expected 3 rows, got $row_count"; return 1; }
  echo "PASS [2/4] Write: $row_count rows + extensions (vector, age)" >> "$report"

  # ── Step 3: Stop + restart — data must persist ──────────────────────────────
  log "  [3/4] Stop + restart — verify data persistence..."
  docker stop "$container" >/dev/null
  docker rm "$container" >/dev/null

  docker run -d --name "$container" \
    -v "${volume}:${mount_path}" \
    -e POSTGRES_USER=edgequake \
    -e POSTGRES_PASSWORD="$PGPASSWORD" \
    -e POSTGRES_DB=edgequake \
    "$image" >/dev/null

  for i in $(seq 1 60); do
    if docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
      psql -U edgequake -d edgequake -c 'SELECT 1' >/dev/null 2>&1; then
      break
    fi
    if [ "$i" -eq 60 ]; then
      log "FAIL: container did not restart"
      docker logs "$container" 2>&1 | tail -20
      echo "FAIL [3/4] Container did not restart" >> "$report"
      return 1
    fi
    sleep 1
  done

  row_count_after=$(docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -tAc "SELECT count(*) FROM persistence_test;")
  [ "$row_count_after" = "3" ] || {
    log "FAIL: expected 3 rows after restart, got $row_count_after"
    echo "FAIL [3/4] Data lost: expected 3, got $row_count_after" >> "$report"
    return 1
  }

  ext_check=$(docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -tAc \
    "SELECT count(*) FROM pg_extension WHERE extname IN ('vector','age');")
  [ "$ext_check" = "2" ] || {
    log "FAIL: extensions missing after restart (got $ext_check)"
    echo "FAIL [3/4] Extensions lost after restart" >> "$report"
    return 1
  }
  echo "PASS [3/4] Restart: $row_count_after rows + $ext_check extensions persisted" >> "$report"

  # ── Step 4: No crash-loop — container stays healthy ─────────────────────────
  log "  [4/4] Health check — no crash loop..."
  sleep 3
  status=$(docker inspect --format='{{.State.Status}}' "$container" 2>/dev/null)
  [ "$status" = "running" ] || {
    log "FAIL: container status is '$status' (expected 'running')"
    docker logs "$container" 2>&1 | tail -20
    echo "FAIL [4/4] Container status: $status" >> "$report"
    return 1
  }
  echo "PASS [4/4] Healthy: container status = $status (no crash loop)" >> "$report"

  echo "" >> "$report"
  echo "RESULT: ALL PASSED" >> "$report"
  log "  ✓ $profile — ALL 4 CHECKS PASSED (mount at $mount_path)"
}

# ── Main ──────────────────────────────────────────────────────────────────────
log "SPEC-042/017 Volume Mount Proof (#280)"
log ""

docker ps -aq --filter "name=eq-vol-proof-" 2>/dev/null | xargs -r docker rm -f >/dev/null 2>&1 || true

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

echo ""
log "=========================================="
log "SUMMARY — Volume Mount Proof (#280)"
log "=========================================="
for f in "$REPORT_DIR"/*-volume-report.txt; do
  [ -f "$f" ] || continue
  profile=$(basename "$f" -volume-report.txt)
  result=$(grep "^RESULT:" "$f" 2>/dev/null | head -1)
  echo "  $profile: ${result:-UNKNOWN}"
done
echo ""

if [ "$failed" -ne 0 ]; then
  log "✗ SOME PROFILES FAILED"
  exit 1
fi

log "✓ VOLUME MOUNT PROOF — ALL PROFILES PASSED (#280 fixed)"
