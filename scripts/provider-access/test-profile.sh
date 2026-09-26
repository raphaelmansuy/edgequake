#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export ROOT_DIR
COMPOSE_FILE="$ROOT_DIR/scripts/provider-access/compose.yaml"

: "${PROFILE:?PROFILE is required (P0|P1|P2a|P2b|P3|P4)}"
: "${SUITE:?SUITE is required (smoke|contracts|recovery|full)}"

case "$PROFILE" in
  P0|P1|P2a|P2b|P3|P4) ;;
  *)
    echo "provider-access: invalid PROFILE=$PROFILE" >&2
    exit 64
    ;;
esac

case "$SUITE" in
  smoke|contracts|recovery|full) ;;
  *)
    echo "provider-access: invalid SUITE=$SUITE" >&2
    exit 64
    ;;
esac

umask 077
RUN_ID="$(date -u +%Y%m%dt%H%M%Sz)-$$-$(od -An -N4 -tx1 /dev/urandom | tr -d ' \n')"
export RUN_ID
export COMPOSE_PROJECT_NAME="eq-pa-$RUN_ID"
export PROVIDER_ACCESS_DB_NAME="eq_pa_${RUN_ID//-/_}"
export PROVIDER_ACCESS_POSTGRES_PASSWORD="provider-access-test"

ARTIFACT_DIR="/tmp/eq-pa-$RUN_ID"
MANIFEST_PATH="$ARTIFACT_DIR/manifest.json"
OWNER_MARKER="$ARTIFACT_DIR/owner"
mkdir "$ARTIFACT_DIR"
printf '%s\n%s\n' "$RUN_ID" "$COMPOSE_PROJECT_NAME" >"$OWNER_MARKER"

MANIFEST_STATUS="running"
MANIFEST_ERROR=""
POSTGRES_VERSION=""
POSTGRES_IMAGE_ID=""
AGE_VERSION=""
PGVECTOR_VERSION=""
TESTS_SELECTED=0
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_SKIPPED=0
CERTIFICATION_SUCCESSES=0
ACTUAL_TEST_IDS=""
COMPOSE_STARTED=0

write_manifest() {
  export MANIFEST_STATUS MANIFEST_ERROR POSTGRES_VERSION POSTGRES_IMAGE_ID
  export AGE_VERSION PGVECTOR_VERSION
  export TESTS_SELECTED TESTS_PASSED TESTS_FAILED TESTS_SKIPPED
  export CERTIFICATION_SUCCESSES ACTUAL_TEST_IDS ARTIFACT_DIR MANIFEST_PATH
  export PROFILE SUITE
  python3 - <<'PY'
import json
import os
import pathlib
import subprocess

root = pathlib.Path(os.environ["ROOT_DIR"])

def git(*args):
    try:
        return subprocess.check_output(
            ["git", "-C", str(root), *args], text=True, stderr=subprocess.DEVNULL
        ).strip()
    except Exception:
        return ""

test_ids = [item for item in os.environ.get("ACTUAL_TEST_IDS", "").split(",") if item]
manifest = {
    "schema_version": 1,
    "run_id": os.environ["RUN_ID"],
    "compose_project_name": os.environ["COMPOSE_PROJECT_NAME"],
    "profile": os.environ["PROFILE"],
    "suite": os.environ["SUITE"],
    "status": os.environ["MANIFEST_STATUS"],
    "error": os.environ.get("MANIFEST_ERROR") or None,
    "source": {
        "commit": git("rev-parse", "HEAD"),
        "dirty": bool(git("status", "--porcelain")),
    },
    "providers": {
        "relational": {
            "kind": "postgres",
            "server_version": os.environ.get("POSTGRES_VERSION") or None,
            "image_id": os.environ.get("POSTGRES_IMAGE_ID") or None,
        },
        "graph": {
            "kind": "age",
            "server_version": os.environ.get("AGE_VERSION") or None,
        },
        "vector": {
            "kind": "pgvector",
            "server_version": os.environ.get("PGVECTOR_VERSION") or None,
        },
    },
    "feature_flags": ["postgres"],
    "tests": {
        "selected": int(os.environ["TESTS_SELECTED"]),
        "passed": int(os.environ["TESTS_PASSED"]),
        "failed": int(os.environ["TESTS_FAILED"]),
        "skipped": int(os.environ["TESTS_SKIPPED"]),
        "certification_successes": int(os.environ["CERTIFICATION_SUCCESSES"]),
        "actual_ids": test_ids,
    },
}
pathlib.Path(os.environ["MANIFEST_PATH"]).write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n"
)
PY
}

marker_matches() {
  [[ -f "$OWNER_MARKER" ]] &&
    [[ "$(sed -n '1p' "$OWNER_MARKER")" == "$RUN_ID" ]] &&
    [[ "$(sed -n '2p' "$OWNER_MARKER")" == "$COMPOSE_PROJECT_NAME" ]]
}

compose() {
  docker compose -f "$COMPOSE_FILE" "$@"
}

cleanup() {
  local rc=$?
  trap - EXIT INT TERM

  if [[ "$rc" -ne 0 && "$MANIFEST_STATUS" == "running" ]]; then
    MANIFEST_STATUS="failed"
    MANIFEST_ERROR="runner_failed"
    TESTS_PASSED=0
    TESTS_FAILED="$TESTS_SELECTED"
    CERTIFICATION_SUCCESSES=0
    write_manifest || true
  fi

  if [[ "$COMPOSE_STARTED" -eq 1 ]]; then
    if marker_matches; then
      compose down --volumes --remove-orphans >/dev/null 2>&1 || true
    else
      echo "provider-access: refusing cleanup; ownership marker mismatch" >&2
      rc=1
    fi
  fi

  echo "provider-access: manifest=$MANIFEST_PATH"
  exit "$rc"
}
trap cleanup EXIT INT TERM

fail_zero() {
  MANIFEST_STATUS="failed"
  MANIFEST_ERROR="$1"
  TESTS_PASSED=0
  TESTS_FAILED="$TESTS_SELECTED"
  CERTIFICATION_SUCCESSES=0
  write_manifest
  echo "provider-access: $1; ZERO certification successes" >&2
  exit 1
}

write_manifest

for command in docker cargo python3; do
  command -v "$command" >/dev/null 2>&1 ||
    fail_zero "missing_required_command_$command"
done
docker compose version >/dev/null 2>&1 ||
  fail_zero "missing_required_service_docker_compose"
docker info >/dev/null 2>&1 ||
  fail_zero "missing_required_service_docker_daemon"

# J01 certifies only the existing P0 composition. Future service mappings are
# intentionally explicit so J17/J19/J22 can enable them without implicit
# provider fallback:
#   P1  -> postgres qdrant
#   P2a -> postgres neo4j
#   P2b -> postgres qdrant neo4j
#   P3  -> qdrant neo4j (SQLite authority arrives in J22)
#   P4  -> postgres neo4j (standalone pgvector + SQLite authority arrives later)
if [[ "$PROFILE" != "P0" || ! "$SUITE" =~ ^(smoke|recovery)$ ]]; then
  fail_zero "profile_suite_not_yet_certifiable"
fi

TESTS_SELECTED=1
if [[ "$SUITE" == "smoke" ]]; then
  ACTUAL_TEST_IDS="PROVIDER-ACCESS-E2E01-SMOKE"
else
  # P0 E2E04 B1–B3 crash/replay barriers only (not full HTTP E2E01–15).
  ACTUAL_TEST_IDS="PROVIDER-ACCESS-E2E04"
fi
write_manifest

COMPOSE_STARTED=1
compose up -d postgres

POSTGRES_CONTAINER="$(compose ps -q postgres)"
[[ -n "$POSTGRES_CONTAINER" ]] ||
  fail_zero "missing_required_service_postgres"

POSTGRES_READY=0
for _ in $(seq 1 90); do
  health="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}missing{{end}}' "$POSTGRES_CONTAINER" 2>/dev/null || true)"
  if [[ "$health" == "healthy" ]]; then
    POSTGRES_READY=1
    break
  fi
  if [[ "$health" == "unhealthy" || "$health" == "missing" ]]; then
    break
  fi
  sleep 2
done
[[ "$POSTGRES_READY" -eq 1 ]] ||
  fail_zero "missing_required_service_postgres_health"

PORT_MAPPING="$(compose port postgres 5432)"
POSTGRES_PORT="${PORT_MAPPING##*:}"
[[ "$POSTGRES_PORT" =~ ^[0-9]+$ ]] ||
  fail_zero "missing_required_service_postgres_port"

# The entrypoint creates this run-scoped database. Keep an explicit idempotent
# creation guard for custom/pre-warmed images.
DB_EXISTS="$(compose exec -T postgres psql -U provider_access -d postgres -tAc \
  "SELECT 1 FROM pg_database WHERE datname = '$PROVIDER_ACCESS_DB_NAME'" | tr -d '[:space:]')"
if [[ "$DB_EXISTS" != "1" ]]; then
  compose exec -T postgres createdb -U provider_access "$PROVIDER_ACCESS_DB_NAME"
fi

unset DATABASE_URL DATABASE_READ_URL EDGEQUAKE_TEST_DATABASE_URL
export DATABASE_URL="postgresql://provider_access:${PROVIDER_ACCESS_POSTGRES_PASSWORD}@127.0.0.1:${POSTGRES_PORT}/${PROVIDER_ACCESS_DB_NAME}"
export EDGEQUAKE_PROVIDER_ACCESS_E2E=1
export EDGEQUAKE_REQUIRE_PROVIDER_ACCESS=1
export EDGEQUAKE_REQUIRE_POSTGRES_TESTS=1
export EDGEQUAKE_ALLOW_MOCK_PROVIDER=1
export EDGEQUAKE_LLM_PROVIDER=mock
export EDGEQUAKE_EMBEDDING_PROVIDER=mock

POSTGRES_VERSION="$(compose exec -T postgres psql -U provider_access \
  -d "$PROVIDER_ACCESS_DB_NAME" -tAc 'SHOW server_version' | tr -d '\r' | xargs)"
AGE_VERSION="$(compose exec -T postgres psql -U provider_access \
  -d "$PROVIDER_ACCESS_DB_NAME" -tAc \
  "SELECT default_version FROM pg_available_extensions WHERE name = 'age'" | tr -d '\r' | xargs)"
PGVECTOR_VERSION="$(compose exec -T postgres psql -U provider_access \
  -d "$PROVIDER_ACCESS_DB_NAME" -tAc \
  "SELECT default_version FROM pg_available_extensions WHERE name = 'vector'" | tr -d '\r' | xargs)"
POSTGRES_IMAGE_ID="$(docker inspect --format '{{.Image}}' "$POSTGRES_CONTAINER")"
write_manifest

(
  cd "$ROOT_DIR/edgequake"
  cargo run -p edgequake --features postgres -- migrate
)

TEST_LOG="$ARTIFACT_DIR/provider_access_e2e.log"
if [[ "$SUITE" == "smoke" ]]; then
  TEST_COMMAND=(cargo test -p edgequake-api --features postgres \
    --test provider_access_e2e -- --nocapture)
else
  # E2E04 B1–B3: ingestion rollback + process-kill replay (requires fault feature).
  TEST_COMMAND=(cargo test -p edgequake-storage \
    --features "postgres,provider-access-fault" \
    --test e2e_spec149_ingestion_committer \
    --test e2e_spec149_process_kill \
    -- --nocapture --test-threads=1)
fi
if (
  cd "$ROOT_DIR/edgequake"
  "${TEST_COMMAND[@]}"
) 2>&1 | tee "$TEST_LOG"; then
  # Derive counts from cargo summary lines when present.
  PASSED_LINE="$(rg -o '([0-9]+) passed' "$TEST_LOG" | tail -1 || true)"
  FAILED_LINE="$(rg -o '([0-9]+) failed' "$TEST_LOG" | tail -1 || true)"
  TESTS_PASSED="${PASSED_LINE%% *}"
  TESTS_PASSED="${TESTS_PASSED:-1}"
  TESTS_FAILED="${FAILED_LINE%% *}"
  TESTS_FAILED="${TESTS_FAILED:-0}"
  TESTS_SKIPPED=0
  if rg -q '^test .* \.\.\. FAILED' "$TEST_LOG"; then
    TESTS_FAILED=1
  fi
  if [[ "$TESTS_FAILED" != "0" ]]; then
    CERTIFICATION_SUCCESSES=0
    MANIFEST_STATUS="failed"
    MANIFEST_ERROR="provider_access_hotpath_failed"
    write_manifest
    exit 1
  fi
  CERTIFICATION_SUCCESSES=1
  MANIFEST_STATUS="passed"
  MANIFEST_ERROR=""
  write_manifest
else
  TESTS_PASSED=0
  TESTS_FAILED=1
  CERTIFICATION_SUCCESSES=0
  MANIFEST_STATUS="failed"
  MANIFEST_ERROR="provider_access_e2e_failed"
  write_manifest
  exit 1
fi
