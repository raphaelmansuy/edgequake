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
RUN_REPORTS=()

# Forbidden host ports (SPEC-93 isolation)
FORBIDDEN_PORTS="8787 55432 8080 5173 8000 5433 9000 9001 3100 5001"
# Stay below the default ephemeral range (32768–60999). Ports inside that
# range look free to a LISTEN probe while a client socket or TIME_WAIT still
# owns them, and `docker run -p` then fails with "address already in use".
PORT_MIN=20000
PORT_MAX=32000

need() { command -v "$1" >/dev/null || { echo "missing $1"; exit 1; }; }
need docker
need sqlx
need git
need python3
need psql
need pg_dump

port_is_free() {
  local p="$1"
  if echo " $FORBIDDEN_PORTS " | grep -q " $p "; then
    return 1
  fi
  # Bind the same way Docker publishes (0.0.0.0, no SO_REUSEADDR) so a port
  # held by an ephemeral client socket is not treated as free.
  python3 - "$p" <<'PY'
import socket, sys
port = int(sys.argv[1])
sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
try:
    sock.bind(("0.0.0.0", port))
except OSError:
    sys.exit(1)
finally:
    sock.close()
PY
}

pick_port() {
  local p
  for _ in $(seq 1 50); do
    p=$((PORT_MIN + RANDOM % (PORT_MAX - PORT_MIN + 1)))
    if port_is_free "$p"; then
      echo "$p"
      return 0
    fi
  done
  echo "no free port in ${PORT_MIN}-${PORT_MAX}" >&2
  return 1
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
  # v0.32 and v0.33 replay migration 167, which writes ag_catalog. Product
  # images from v0.35.5 do not ship Apache AGE, so this matrix stays on the
  # last image that does. Do not build the current Dockerfiles here.
  local remote="ghcr.io/raphaelmansuy/edgequake-postgres:0.35.4"
  if [[ "$major" == "16" ]]; then
    remote="ghcr.io/raphaelmansuy/edgequake-postgres:0.35.4-pg16"
  elif [[ "$major" == "17" ]]; then
    remote="ghcr.io/raphaelmansuy/edgequake-postgres:0.35.4-pg17"
  fi
  echo "Pulling $remote for epoch replay (Apache AGE required)…" >&2
  docker pull "$remote" >&2
  docker tag "$remote" "$tag"
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

wait_pg() {
  local name="$1" port="$2" url state
  url=$(db_url "$port")
  local _
  for _ in $(seq 1 90); do
    state=$(docker inspect -f '{{.State.Status}}' "$name" 2>/dev/null || echo missing)
    if [[ "$state" == "exited" || "$state" == "dead" || "$state" == "missing" ]]; then
      echo "container exited: $name" >&2
      docker logs "$name" >&2 || true
      return 1
    fi
    # docker exec can succeed before the published host port accepts connections.
    if docker exec -e PGPASSWORD=edgequake "$name" \
      psql -U edgequake -d edgequake -v ON_ERROR_STOP=1 -c 'SELECT 1' >/dev/null 2>&1 \
      && psql "$url" -v ON_ERROR_STOP=1 -c 'SELECT 1' >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  echo "postgres not ready: $name" >&2
  docker logs "$name" >&2 || true
  return 1
}

# stdout: "<port> <container>". Retries a new host port when Docker cannot bind.
start_pg() {
  local major="$1"
  local prefer="${2:-}"
  local img attempt port name err
  img=$(ensure_pg_image "$major")
  for attempt in 1 2 3 4 5 6; do
    if [[ "$attempt" -eq 1 && -n "$prefer" ]]; then
      port="$prefer"
    else
      port=$(pick_port) || return 1
    fi
    name="eq150-pg${major}-${port}"
    docker rm -f "$name" >/dev/null 2>&1 || true
    err=$(mktemp)
    if docker run -d --name "$name" \
      -e POSTGRES_USER=edgequake -e POSTGRES_PASSWORD=edgequake -e POSTGRES_DB=edgequake \
      -p "127.0.0.1:${port}:5432" "$img" >/dev/null 2>"$err"; then
      rm -f "$err"
      if wait_pg "$name" "$port"; then
        echo "$port $name"
        return 0
      fi
      echo "postgres not ready: $name (attempt $attempt)" >&2
    else
      echo "docker run failed for $name (attempt $attempt)" >&2
      cat "$err" >&2 || true
      rm -f "$err"
    fi
    docker rm -f "$name" >/dev/null 2>&1 || true
    sleep 1
  done
  echo "PG start failed for major=$major" >&2
  return 1
}

db_url() {
  local port="$1"
  echo "postgres://edgequake:edgequake@127.0.0.1:${port}/edgequake"
}

replay_epoch() {
  local tag="$1" url="$2"
  local tmp rc=0
  tmp=$(mktemp -d)
  # run_one calls this under `set +e`. A trailing `rm` would hide sqlx's status.
  git archive "$tag" edgequake/migrations | tar -x -C "$tmp" || rc=$?
  if [[ $rc -eq 0 ]]; then
    (
      cd "$tmp/edgequake"
      DATABASE_URL="$url" sqlx migrate run --source migrations
    ) || rc=$?
  fi
  rm -rf "$tmp"
  return "$rc"
}

run_one() {
  local tag="$1" major="$2" mode="$3"
  local port container url bin report_dir t0 t1 status started
  report_dir="$REPORT_ROOT/pg${major}"
  RUN_REPORTS+=("$report_dir/${tag}.json")
  mkdir -p "$report_dir"
  # Command substitution does not abort under `set -e` when start_pg fails.
  started=$(start_pg "$major") || true
  if [[ -z "$started" ]]; then
    echo "PG start failed for $tag on PG$major" >&2
    status="pg_start_fail"
    port=""
    container=""
    url=""
  else
    port="${started%% *}"
    container="${started#* }"
    url=$(db_url "$port")
    status="ok"
  fi
  bin=$(resolve_head_bin)

  # FORCE_REPLAY=1 skips GHCR image boots (deterministic checksum replay).
  if [[ "${FORCE_REPLAY:-0}" == "1" ]]; then
    mode="replay"
  fi

  echo "=== $tag → HEAD on PG$major (mode=$mode, port=${port:-none}) ==="
  t0=$(date +%s)

  set +e
  if [[ "$status" != "ok" ]]; then
    :
  else
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
          started=$(start_pg "$major") || true
          if [[ -n "$started" ]]; then
            port="${started%% *}"
            container="${started#* }"
            url=$(db_url "$port")
            replay_epoch "$tag" "$url" || true
            psql "$url" -v ON_ERROR_STOP=0 -f "$SEED_SQL" >/dev/null 2>&1 || true
          else
            container=""
            echo "  PG recreate failed" >&2
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
    local fresh_port fresh_c fresh_url fresh_started
    # Under `set +e`, a failed `$(start_pg)` leaves an empty name. start_pg
    # already retries bind failures; one extra call covers a dead daemon.
    fresh_started=$(start_pg "$major") || true
    if [[ -z "$fresh_started" ]]; then
      echo "fresh PG start failed for schema dump (tag=$tag pg=$major); retrying once" >&2
      fresh_started=$(start_pg "$major") || true
    fi
    if [[ -z "$fresh_started" ]]; then
      echo "fresh PG start failed again for schema dump (tag=$tag pg=$major)" >&2
      status="dump_fail"
      fresh_port=""
      fresh_c=""
    else
      fresh_port="${fresh_started%% *}"
      fresh_c="${fresh_started#* }"
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
    if [[ -n "$fresh_c" ]]; then
      docker rm -f "$fresh_c" >/dev/null 2>&1 || true
    fi
    fi
  fi
  fi
  set -e

  t1=$(date +%s)
  local dur=$((t1 - t0))
  local ledger_max=0
  if [[ -n "$url" ]]; then
    ledger_max=$(psql "$url" -Atc "SELECT COALESCE(MAX(version),0) FROM _sqlx_migrations WHERE success" 2>/dev/null || echo 0)
  fi

  python3 - "$report_dir/${tag}.json" "$tag" "$major" "$status" "$dur" "$ledger_max" <<'PY'
import json,sys
path,tag,major,status,dur,ledger=sys.argv[1:7]
json.dump({
  "tag": tag, "pg": int(major), "status": status,
  "duration_s": int(dur), "ledger_max": int(ledger),
}, open(path,"w"), indent=2)
print(f"  → status={status} duration={dur}s ledger_max={ledger}")
PY

  if [[ -n "$container" ]]; then
    docker rm -f "$container" >/dev/null 2>&1 || true
  fi
  [[ "$status" == "ok" ]]
}

# True once some other session holds edgequake.migrate.run.
# pg_try_advisory_lock returns true when *this* probe acquired it; that
# session exits and releases, and the probe keeps waiting.
wait_until_migrate_lock_held() {
  local url="$1" hold_pid="$2"
  local i got
  for i in $(seq 1 40); do
    if ! kill -0 "$hold_pid" 2>/dev/null; then
      echo "  → FAIL lock holder exited before the lock was visible"
      return 1
    fi
    got=$(psql "$url" -Atqc "SELECT pg_try_advisory_lock(hashtext('edgequake.migrate.run'))" 2>/dev/null || true)
    got="${got//[[:space:]]/}"
    if [[ "$got" == "f" ]]; then
      echo "  → run lock is held (probe $i)"
      return 0
    fi
    sleep 0.25
  done
  echo "  → FAIL advisory lock was not held after 10s"
  return 1
}

# Chaos: concurrent migrate → one exit 75.
# The second migrate is the default command (AGE stays). It must still exit 75
# when only the AGE drop remains, which is the steady state after --confirm-drop.
chaos_lock() {
  local major=16
  local port container url bin hold_pid rc hold_sql started
  started=$(start_pg "$major") || true
  if [[ -z "$started" ]]; then
    echo "  → FAIL chaos PG start"
    return 1
  fi
  port="${started%% *}"
  container="${started#* }"
  url=$(db_url "$port")
  bin=$(resolve_head_bin)
  echo "=== chaos: concurrent migrate lock ==="
  if ! DATABASE_URL="$url" "$bin" migrate --confirm-drop >/tmp/eq150-chaos-prep.log 2>&1; then
    echo "  → FAIL preparatory migrate"
    cat /tmp/eq150-chaos-prep.log
    docker rm -f "$container" >/dev/null 2>&1 || true
    return 1
  fi
  # One DO block: lock, then sleep, on a session that stays open.
  hold_sql=$(cat <<'SQL'
DO $$
BEGIN
  PERFORM pg_advisory_lock(hashtext('edgequake.migrate.run'));
  PERFORM pg_sleep(90);
END
$$;
SQL
)
  psql "$url" -v ON_ERROR_STOP=1 -c "$hold_sql" >/tmp/eq150-chaos-holder.log 2>&1 &
  hold_pid=$!
  if ! wait_until_migrate_lock_held "$url" "$hold_pid"; then
    echo "----- /tmp/eq150-chaos-holder.log -----"
    cat /tmp/eq150-chaos-holder.log 2>/dev/null || true
    kill "$hold_pid" 2>/dev/null || true
    wait "$hold_pid" 2>/dev/null || true
    docker rm -f "$container" >/dev/null 2>&1 || true
    return 1
  fi
  set +e
  EDGEQUAKE_MIGRATE_LOCK_DEADLINE=5 DATABASE_URL="$url" "$bin" migrate >/tmp/eq150-chaos-lock.log 2>&1
  rc=$?
  set -e
  kill "$hold_pid" 2>/dev/null || true
  wait "$hold_pid" 2>/dev/null || true
  docker rm -f "$container" >/dev/null 2>&1 || true
  if grep -q "MIGRATE_LOCK_BUSY\|EX_TEMPFAIL\|tempfail" /tmp/eq150-chaos-lock.log || [[ $rc -eq 75 ]]; then
    echo "  → PASS concurrent lock (rc=$rc)"
    return 0
  fi
  echo "  → FAIL concurrent lock (rc=$rc)"
  echo "----- /tmp/eq150-chaos-lock.log -----"
  cat /tmp/eq150-chaos-lock.log 2>/dev/null || true
  echo "----- /tmp/eq150-chaos-holder.log -----"
  cat /tmp/eq150-chaos-holder.log 2>/dev/null || true
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

# SKIP_CHAOS=1: epoch-only (useful when looping EPOCH=…); default still runs chaos.
if [[ "$QUICK" != "1" && "${SKIP_CHAOS:-0}" != "1" ]]; then
  chaos_lock || FAILS=$((FAILS + 1))
fi

# Summarize
echo ""
echo "=== SPEC-150 epoch matrix summary ==="
python3 - "${RUN_REPORTS[@]}" <<'PY' || true
import json, sys
from pathlib import Path
oks = fails = 0
for path in sys.argv[1:]:
    try:
        o = json.loads(Path(path).read_text())
    except Exception as error:
        fails += 1
        print("FAIL", path, str(error))
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
