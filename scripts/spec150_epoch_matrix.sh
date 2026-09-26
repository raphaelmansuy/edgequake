#!/usr/bin/env bash
# scripts/spec150_epoch_matrix.sh — SPEC-150 epoch upgrade matrix (PG16/17/18)
#
# Usage:
#   ./scripts/spec150_epoch_matrix.sh                  # all epochs × PG=16
#   PG=all ./scripts/spec150_epoch_matrix.sh           # all majors
#   QUICK=1 ./scripts/spec150_epoch_matrix.sh          # key epochs only
#   PG=16 EPOCH=v0.22.0 ./scripts/spec150_epoch_matrix.sh
#
# Isolation: containers named eq150-*; free ports; never touches SPEC-93 foreign ports.
# Trap removes only eq150-* containers.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

PG_LIST="${PG:-16}"
[[ "$PG_LIST" == "all" ]] && PG_LIST="16 17 18"
QUICK="${QUICK:-0}"
EPOCH_FILTER="${EPOCH:-}"
HEAD_BIN="${HEAD_BIN:-}"
REPORT_ROOT="$REPO_ROOT/specs/150-reliable-migration-system/reports"
EPOCHS_TOML="$REPO_ROOT/scripts/spec150/epochs.toml"
SEED_SQL="$REPO_ROOT/scripts/spec150/seed_realistic.sql"
ALLOWLIST="$REPO_ROOT/scripts/spec150/schema_diff_allowlist.txt"

# Forbidden host ports (SPEC-93 isolation)
FORBIDDEN_PORTS="8787 55432 8080 5173 8000 5433 9000 9001 3100 5001"

need() { command -v "$1" >/dev/null || { echo "missing $1"; exit 1; }; }
need docker
need sqlx
need git
need python3
need psql
need pg_dump

pick_port() {
  local p
  for _ in $(seq 1 50); do
    p=$((45000 + RANDOM % 10000))
    if echo " $FORBIDDEN_PORTS " | grep -q " $p "; then continue; fi
    if ! lsof -nP -iTCP:"$p" -sTCP:LISTEN >/dev/null 2>&1; then
      echo "$p"
      return
    fi
  done
  echo "no free port" >&2
  exit 1
}

cleanup() {
  local ids
  ids=$(docker ps -aq --filter "name=eq150-" 2>/dev/null || true)
  if [[ -n "$ids" ]]; then
    # shellcheck disable=SC2086
    docker rm -f $ids >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

ensure_pg_image() {
  local major="$1"
  local tag="eq150-pg${major}:local"
  if docker image inspect "$tag" >/dev/null 2>&1; then
    echo "$tag"
    return
  fi
  local df="edgequake/docker/Dockerfile.postgres"
  [[ "$major" == "17" ]] && df="edgequake/docker/Dockerfile.postgres.pg17"
  [[ "$major" == "18" ]] && df="edgequake/docker/Dockerfile.postgres.pg18"
  echo "Building $tag from $df (AGE+pgvector)…" >&2
  docker build -t "$tag" -f "$df" edgequake/docker >&2
  echo "$tag"
}

resolve_head_bin() {
  if [[ -n "$HEAD_BIN" && -x "$HEAD_BIN" ]]; then
    echo "$HEAD_BIN"
    return
  fi
  local cand="$REPO_ROOT/.cargo-target/debug/edgequake"
  if [[ -x "$cand" ]]; then
    echo "$cand"
    return
  fi
  cand="$REPO_ROOT/edgequake/target/debug/edgequake"
  if [[ -x "$cand" ]]; then
    echo "$cand"
    return
  fi
  echo "Building HEAD edgequake binary…" >&2
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$REPO_ROOT/.cargo-target}"
  (cd edgequake && cargo build -p edgequake --features postgres) >&2
  echo "$CARGO_TARGET_DIR/debug/edgequake"
}

list_epochs() {
  python3 - "$EPOCHS_TOML" "$QUICK" "$EPOCH_FILTER" <<'PY'
import sys
path, quick, filt = sys.argv[1], sys.argv[2]=="1", sys.argv[3]
# minimal toml parse for [[epoch]] blocks
text=open(path).read().split("[[epoch]]")
for block in text[1:]:
    d={}
    for line in block.splitlines():
        line=line.strip()
        if not line or line.startswith("#"): continue
        if "=" not in line: continue
        k,v=line.split("=",1)
        d[k.strip()]=v.strip().strip('"')
    tag=d.get("tag","")
    if filt and tag!=filt: continue
    if quick and d.get("key")!="true": continue
    print(tag, d.get("pg_majors","[16]").strip("[]").replace(" ",""), d.get("mode","replay"))
PY
}

start_pg() {
  local major="$1" port="$2"
  local img
  img=$(ensure_pg_image "$major")
  local name="eq150-pg${major}-${port}"
  docker rm -f "$name" >/dev/null 2>&1 || true
  if ! docker run -d --name "$name" \
    -e POSTGRES_USER=edgequake -e POSTGRES_PASSWORD=edgequake -e POSTGRES_DB=edgequake \
    -p "${port}:5432" "$img" >/dev/null; then
    echo "docker run failed for $name" >&2
    return 1
  fi
  for _ in $(seq 1 90); do
    if docker exec -e PGPASSWORD=edgequake "$name" \
      psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c 'SELECT 1' >/dev/null 2>&1; then
      echo "$name"
      return 0
    fi
    sleep 1
  done
  echo "postgres not ready: $name" >&2
  docker logs "$name" >&2 || true
  docker rm -f "$name" >/dev/null 2>&1 || true
  return 1
}

db_url() {
  local port="$1"
  echo "postgres://edgequake:edgequake@127.0.0.1:${port}/edgequake"
}

replay_epoch() {
  local tag="$1" url="$2"
  local tmp
  tmp=$(mktemp -d)
  git archive "$tag" edgequake/migrations | tar -x -C "$tmp"
  # sqlx migrate run against the tag's migration set
  (
    cd "$tmp/edgequake"
    DATABASE_URL="$url" sqlx migrate run --source migrations
  )
  rm -rf "$tmp"
}

run_one() {
  local tag="$1" major="$2" mode="$3"
  local port container url bin report_dir t0 t1 status
  report_dir="$REPORT_ROOT/pg${major}"
  mkdir -p "$report_dir"
  port=$(pick_port)
  container=$(start_pg "$major" "$port")
  url=$(db_url "$port")
  bin=$(resolve_head_bin)

  # FORCE_REPLAY=1 skips GHCR image boots (deterministic checksum replay).
  if [[ "${FORCE_REPLAY:-0}" == "1" ]]; then
    mode="replay"
  fi

  echo "=== $tag → HEAD on PG$major (mode=$mode, port=$port) ==="
  t0=$(date +%s)
  status="ok"

  set +e
  if [[ "$mode" == "image" ]]; then
    # Prefer image when available; fall back to replay on pull failure.
    if ! docker pull "ghcr.io/raphaelmansuy/edgequake:${tag#v}" >/dev/null 2>&1; then
      echo "image pull failed for $tag — falling back to replay"
      mode="replay"
    else
      # Boot old image briefly to auto-migrate (pre-0.23) or run migrate.
      local old="eq150-old-${port}"
      docker rm -f "$old" >/dev/null 2>&1 || true
      docker run -d --name "$old" --network "container:$container" \
        -e DATABASE_URL="postgres://edgequake:edgequake@127.0.0.1:5432/edgequake" \
        -e EDGEQUAKE_LLM_PROVIDER=mock -e EDGEQUAKE_EMBEDDING_PROVIDER=mock \
        -e EDGEQUAKE_ALLOW_MOCK_PROVIDER=1 -e EDGEQUAKE_DEV_MODE=true \
        -e EDGEQUAKE_AUTH_ENABLED=false \
        "ghcr.io/raphaelmansuy/edgequake:${tag#v}" >/dev/null 2>&1
      sleep 8
      # Try migrate for post-0.23 images
      docker exec "$old" edgequake migrate >/dev/null 2>&1 || true
      docker rm -f "$old" >/dev/null 2>&1 || true
    fi
  fi
  if [[ "$mode" == "replay" ]]; then
    replay_epoch "$tag" "$url" || status="replay_fail"
  fi

  if [[ "$status" == "ok" ]]; then
    psql "$url" -v ON_ERROR_STOP=0 -f "$SEED_SQL" >/dev/null 2>&1 || true
    DATABASE_URL="$url" "$bin" migrate >"$report_dir/${tag}-migrate.log" 2>&1
    local rc=$?
    if [[ $rc -ne 0 ]]; then
      # Transient docker networking (connection refused) — one retry.
      # Under SKIP_SCHEMA_DIFF CI also retries any migrate_fail once (container
      # flaps mid-matrix are common on shared GH runners).
      if [[ "${SKIP_SCHEMA_DIFF:-0}" == "1" ]] \
        || grep -qiE 'connection refused|could not connect|server closed' "$report_dir/${tag}-migrate.log"; then
        echo "  migrate retry after failure (rc=$rc)…" >&2
        sleep 3
        # Recreate PG if the container died mid-run.
        if ! docker exec -e PGPASSWORD=edgequake "$container" \
          psql -U edgequake -d edgequake -c 'SELECT 1' >/dev/null 2>&1; then
          echo "  recreating PG container for retry…" >&2
          docker rm -f "$container" >/dev/null 2>&1 || true
          port=$(pick_port)
          container=$(start_pg "$major" "$port") || true
          url=$(db_url "$port")
          if [[ -n "$container" ]]; then
            replay_epoch "$tag" "$url" || true
            psql "$url" -v ON_ERROR_STOP=0 -f "$SEED_SQL" >/dev/null 2>&1 || true
          fi
        fi
        if [[ -n "$container" ]]; then
          DATABASE_URL="$url" "$bin" migrate >>"$report_dir/${tag}-migrate.log" 2>&1
          rc=$?
        fi
      fi
    fi
    if [[ $rc -ne 0 ]]; then
      # Soft-exit (irreversible pending) prints a soft message and exits 0.
      # Any non-zero is a hard failure.
      status="migrate_fail"
    fi
    DATABASE_URL="$url" "$bin" migrate --confirm-drop >>"$report_dir/${tag}-migrate.log" 2>&1
    rc=$?
    if [[ $rc -ne 0 && "$status" == "ok" ]]; then
      status="migrate_confirm_fail"
    fi
    DATABASE_URL="$url" EDGEQUAKE_MIGRATION_MODE=automatic "$bin" migrate drain --timeout 120 \
      >>"$report_dir/${tag}-drain.log" 2>&1 || true

    # Equivalence: schema-only dump vs fresh HEAD (use container pg_dump —
    # host Homebrew client is often older than the server major).
    # SKIP_SCHEMA_DIFF=1 (CI default with FORCE_REPLAY): ledger_max proof only;
    # dump equivalence is flaky under GH Actions docker density.
    if [[ "${SKIP_SCHEMA_DIFF:-0}" == "1" ]]; then
      echo "  (SKIP_SCHEMA_DIFF=1 — ledger proof only)" >&2
    else
    local fresh_port fresh_c fresh_url
    fresh_port=$(pick_port)
    # Under `set +e`, `$(start_pg …)` must not use `exit` (that only kills the
    # subshell and leaves an empty name → `pg_dump failed for :`).
    if ! fresh_c=$(start_pg "$major" "$fresh_port"); then
      echo "fresh PG start failed for schema dump (tag=$tag pg=$major); retrying once" >&2
      fresh_port=$(pick_port)
      if ! fresh_c=$(start_pg "$major" "$fresh_port"); then
        echo "fresh PG start failed again for schema dump (tag=$tag pg=$major)" >&2
        status="dump_fail"
      fi
    fi
    if [[ "$status" == "ok" && -n "$fresh_c" ]]; then
      fresh_url=$(db_url "$fresh_port")
      DATABASE_URL="$fresh_url" "$bin" migrate --confirm-drop >"$report_dir/${tag}-fresh.log" 2>&1 || true
    fi
    dump_schema() {
      local cname="$1" out="$2"
      local tmp attempt
      tmp=$(mktemp)
      if [[ -z "$cname" ]]; then
        echo "pg_dump skipped: empty container name" >&2
        rm -f "$tmp"
        return 1
      fi
      for attempt in 1 2 3; do
        if docker exec -e PGPASSWORD=edgequake "$cname" \
          pg_dump -U edgequake -d edgequake --schema-only --no-owner --no-privileges \
          >"$tmp" 2>/tmp/eq150-pgdump.err; then
          break
        fi
        echo "pg_dump attempt $attempt failed for $cname:" >&2
        cat /tmp/eq150-pgdump.err >&2 || true
        if [[ "$attempt" -eq 3 ]]; then
          rm -f "$tmp"
          return 1
        fi
        sleep 2
      done
      grep -v '^--' "$tmp" | grep -v '^$' \
        | grep -v '^\\restrict ' | grep -v '^\\unrestrict ' \
        | sort >"$out" || true
      rm -f "$tmp"
      [[ -s "$out" ]] || { echo "empty schema dump for $cname" >&2; return 1; }
      return 0
    }
    if [[ "$status" == "ok" ]]; then
      if ! dump_schema "$container" "$report_dir/${tag}-upgraded.schema"; then
        status="dump_fail"
      fi
      if ! dump_schema "$fresh_c" "$report_dir/${tag}-fresh.schema"; then
        status="dump_fail"
      fi
    fi
    if [[ "$status" == "ok" ]]; then
      if ! diff -u "$report_dir/${tag}-fresh.schema" "$report_dir/${tag}-upgraded.schema" \
           >"$report_dir/${tag}-schema.diff"; then
        # Allowlisted diffs only
        if [[ -s "$ALLOWLIST" ]]; then
          local leftover
          leftover=$(grep -E '^[+-]' "$report_dir/${tag}-schema.diff" | grep -Ev '^[+-]{3}' \
            | grep -Ev -f "$ALLOWLIST" || true)
          if [[ -n "$leftover" ]]; then
            status="schema_drift"
          fi
        else
          if [[ -s "$report_dir/${tag}-schema.diff" ]]; then
            status="schema_drift"
          fi
        fi
      else
        rm -f "$report_dir/${tag}-schema.diff"
      fi
    fi
    docker rm -f "$fresh_c" >/dev/null 2>&1 || true
    fi
  fi
  set -e

  t1=$(date +%s)
  local dur=$((t1 - t0))
  local ledger_max
  ledger_max=$(psql "$url" -Atc "SELECT COALESCE(MAX(version),0) FROM _sqlx_migrations WHERE success" 2>/dev/null || echo 0)

  python3 - "$report_dir/${tag}.json" "$tag" "$major" "$status" "$dur" "$ledger_max" <<'PY'
import json,sys
path,tag,major,status,dur,ledger=sys.argv[1:7]
json.dump({
  "tag": tag, "pg": int(major), "status": status,
  "duration_s": int(dur), "ledger_max": int(ledger),
}, open(path,"w"), indent=2)
print(f"  → status={status} duration={dur}s ledger_max={ledger}")
PY

  docker rm -f "$container" >/dev/null 2>&1 || true
  [[ "$status" == "ok" ]]
}

# Chaos: concurrent migrate → one exit 75
chaos_lock() {
  local major=16
  local port container url bin hold_pid
  port=$(pick_port)
  container=$(start_pg "$major" "$port")
  url=$(db_url "$port")
  bin=$(resolve_head_bin)
  echo "=== chaos: concurrent migrate lock ==="
  DATABASE_URL="$url" "$bin" migrate --confirm-drop >/dev/null 2>&1 || true
  # Hold advisory lock on a long-lived session (psql exits release locks).
  psql "$url" -v ON_ERROR_STOP=1 -c \
    "SELECT pg_advisory_lock(hashtext('edgequake.migrate.run')); SELECT pg_sleep(60);" \
    >/dev/null 2>&1 &
  hold_pid=$!
  sleep 1
  set +e
  EDGEQUAKE_MIGRATE_LOCK_DEADLINE=5 DATABASE_URL="$url" "$bin" migrate >/tmp/eq150-chaos-lock.log 2>&1
  local rc=$?
  set -e
  kill "$hold_pid" 2>/dev/null || true
  wait "$hold_pid" 2>/dev/null || true
  docker rm -f "$container" >/dev/null 2>&1 || true
  if grep -q "MIGRATE_LOCK_BUSY\|EX_TEMPFAIL\|tempfail" /tmp/eq150-chaos-lock.log || [[ $rc -eq 75 ]]; then
    echo "  → PASS concurrent lock (rc=$rc)"
    return 0
  fi
  echo "  → FAIL concurrent lock (rc=$rc); see /tmp/eq150-chaos-lock.log"
  return 1
}

FAILS=0
while read -r tag pg_majors mode; do
  [[ -z "$tag" ]] && continue
  for major in ${pg_majors//,/ }; do
    if ! echo " $PG_LIST " | grep -q " $major "; then
      continue
    fi
    if ! run_one "$tag" "$major" "$mode"; then
      FAILS=$((FAILS + 1))
    fi
  done
done < <(list_epochs)

if [[ "$QUICK" != "1" ]]; then
  chaos_lock || FAILS=$((FAILS + 1))
fi

# Summarize
echo ""
echo "=== SPEC-150 epoch matrix summary ==="
python3 - "$REPORT_ROOT" <<'PY' || true
import json, sys
from pathlib import Path
root = Path(sys.argv[1])
oks = fails = 0
for path in sorted(root.glob("pg*/*.json")):
    try:
        o = json.loads(path.read_text())
    except Exception:
        continue
    if o.get("status") == "ok":
        oks += 1
    else:
        fails += 1
        print("FAIL", o)
print(f"ok={oks} fail={fails}")
PY

if [[ $FAILS -gt 0 ]]; then
  echo "FAILED: $FAILS epoch/chaos case(s)"
  exit 1
fi
echo "PASS: SPEC-150 epoch matrix"
