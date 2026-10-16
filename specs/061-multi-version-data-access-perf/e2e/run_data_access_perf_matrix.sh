#!/usr/bin/env bash
# SPEC-061/062 — DataAccess performance matrix across PG16/17/18.
#
# Usage:
#   ./specs/061-multi-version-data-access-perf/e2e/run_data_access_perf_matrix.sh [pg16|pg17|pg18|all]
#
# Env:
#   EDGEQUAKE_PERF_RELEASE=1     → cargo test --release
#   EDGEQUAKE_PERF_SCALE=prod    → production-shaped stress (50k @1536)
#   EDGEQUAKE_PERF_SCALE=large   → capacity ladder sizes (use make data-access-perf-capacity-ladder)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
DOCKER_DIR="$ROOT/edgequake/docker"
PINS="$DOCKER_DIR/extension-pins.sh"
EDGEQUAKE_DIR="$ROOT/edgequake"
PGPASSWORD="${POSTGRES_PASSWORD:-edgequake_secret}"
PROFILE_ARG="${1:-all}"
PERF_SCALE="$(echo "${EDGEQUAKE_PERF_SCALE:-default}" | tr '[:upper:]' '[:lower:]')"
export EDGEQUAKE_PERF_SCALE="${EDGEQUAKE_PERF_SCALE:-default}"

if [[ "$PERF_SCALE" == "prod" ]]; then
  echo "NOTE: EDGEQUAKE_PERF_SCALE=prod — 50k ANN/FTS @1536, Mix 5k; expect longer wall time."
elif [[ "$PERF_SCALE" == "large" ]]; then
  echo "NOTE: EDGEQUAKE_PERF_SCALE=large — capacity ladder sizes (prefer make data-access-perf-capacity-ladder)."
fi
if [[ "${EDGEQUAKE_PERF_RELEASE:-0}" == "1" || "${EDGEQUAKE_PERF_RELEASE:-}" == "true" ]]; then
  echo "NOTE: EDGEQUAKE_PERF_RELEASE=1 — cargo test --release"
fi

build_image_if_needed() {
  local profile="$1"
  local image="$2"
  # pg18 pins use edgequake-postgres:local; also accept :pg18 if already tagged.
  if docker image inspect "$image" >/dev/null 2>&1; then
    echo "  image $image present"
    return 0
  fi
  if [[ "$profile" == "pg18" ]] && docker image inspect "edgequake-postgres:pg18" >/dev/null 2>&1; then
    echo "  image edgequake-postgres:pg18 present — tagging as $image"
    docker tag "edgequake-postgres:pg18" "$image"
    return 0
  fi
  echo "  building $image …"
  case "$profile" in
    pg16) (cd "$ROOT" && make postgres-image-build) ;;
    pg17) (cd "$ROOT" && make postgres-image-build-pg17) ;;
    pg18) (cd "$ROOT" && make postgres-image-build-pg18) ;;
    *) echo "unknown profile $profile"; return 1 ;;
  esac
}

assert_extensions() {
  local container="$1"
  local age_min="$2"
  local vec_min="$3"
  local versions
  versions="$(docker exec -e PGPASSWORD="$PGPASSWORD" "$container" \
    psql -U edgequake -d edgequake -At -c \
    "SELECT extname||'='||extversion FROM pg_extension WHERE extname IN ('age','vector') ORDER BY 1")"
  echo "  extensions: $versions"
  local age_ver vec_ver
  age_ver="$(echo "$versions" | sed -n 's/^age=//p')"
  vec_ver="$(echo "$versions" | sed -n 's/^vector=//p')"
  if [[ -z "$age_ver" || -z "$vec_ver" ]]; then
    echo "ERROR: age/vector extensions missing"
    return 1
  fi
  if [[ "$(printf '%s\n%s\n' "$age_min" "$age_ver" | sort -V | head -1)" != "$age_min" ]]; then
    echo "ERROR: AGE $age_ver < min $age_min"
    return 1
  fi
  if [[ "$(printf '%s\n%s\n' "$vec_min" "$vec_ver" | sort -V | head -1)" != "$vec_min" ]]; then
    echo "ERROR: pgvector $vec_ver < min $vec_min"
    return 1
  fi
}

run_one_test() {
  local pkg="$1"
  local features="$2"
  local test="$3"
  local log="$4"
  local release_args=()
  if [[ "${EDGEQUAKE_PERF_RELEASE:-0}" == "1" || "${EDGEQUAKE_PERF_RELEASE:-}" == "true" ]]; then
    release_args=(--release)
  fi
  echo "  → $pkg --test $test ${release_args[*]:-}"
  if [[ -n "$features" ]]; then
    cargo test -p "$pkg" --features "$features" "${release_args[@]}" --test "$test" -- --nocapture 2>&1 | tee -a "$log"
  else
    cargo test -p "$pkg" "${release_args[@]}" --test "$test" -- --nocapture 2>&1 | tee -a "$log"
  fi
}

run_cargo_gates() {
  local profile="$1"
  local report="/tmp/eq-perf-${profile}.jsonl"
  local log="/tmp/eq-perf-${profile}-cargo.log"
  : >"$report"
  : >"$log"

  cd "$EDGEQUAKE_DIR"

  local storage_tests=(
    e2e_spec054_age_pgvector_perf
    e2e_spec054_mix_scale_perf
    e2e_spec059_halfvec_perf_recall
    e2e_spec059_hnsw_indexdef_ef64
    e2e_spec060_fts_perf_explain
    e2e_spec060_ingest_stage_perf
    e2e_spec060_compensate_retract_perf
    e2e_spec061_kv_access_perf
    e2e_spec061_vector_unfiltered_ann
    e2e_spec061_edge_upsert_perf
    e2e_spec061_degrees_batch_perf
    e2e_spec061_stress_concurrent_ann
    e2e_spec061_stress_concurrent_fts
    e2e_spec061_stress_concurrent_expand
    e2e_spec061_stress_pool_saturation
  )
  for t in "${storage_tests[@]}"; do
    run_one_test edgequake-storage postgres "$t" "$log"
  done

  local query_tests=(
    e2e_spec061_query_engine_postgres_arms
    e2e_spec061_stress_concurrent_mix
  )
  for t in "${query_tests[@]}"; do
    run_one_test edgequake-query postgres "$t" "$log"
  done

  # API gates — hard-fail under REQUIRE_POSTGRES (List reconcile + query smoke).
  run_one_test edgequake-api postgres e2e_spec054_documents_list_perf "$log"
  run_one_test edgequake-api "" e2e_spec054_query_perf_smoke "$log"

  grep -E '^PERF_REPORT ' "$log" | sed 's/^PERF_REPORT //' >>"$report" || true

  if grep -E 'SKIP:.*(DATABASE_URL|POSTGRES_PASSWORD)' "$log" >/dev/null 2>&1; then
    echo "ERROR: DATABASE soft-skip under REQUIRE_POSTGRES"
    return 1
  fi
  echo "  report → $report ($(wc -l <"$report" | tr -d ' ') lines)"
}

run_profile() {
  local profile="$1"
  # shellcheck source=/dev/null
  EQ_POSTGRES_PROFILE="$profile" source "$PINS"
  local image="$EQ_POSTGRES_IMAGE_TAG"

  echo ""
  echo "========== DATA-ACCESS PERF: $profile ($image) =========="
  build_image_if_needed "$profile" "$image"

  # SPEC-065 DRY: shared ephemeral PG (shm + shared_buffers). Prod needs residency
  # headroom for 50k@1536 concurrent stress (cold cliff otherwise).
  if [[ "$PERF_SCALE" == "prod" || "$PERF_SCALE" == "large" ]]; then
    export EQ_EPHEMERAL_PG_SHM="${EQ_EPHEMERAL_PG_SHM:-4g}"
    export EQ_EPHEMERAL_PG_SHARED_BUFFERS="${EQ_EPHEMERAL_PG_SHARED_BUFFERS:-2GB}"
    export EQ_EPHEMERAL_PG_MAINTENANCE_WORK_MEM="${EQ_EPHEMERAL_PG_MAINTENANCE_WORK_MEM:-1GB}"
  fi
  # shellcheck source=/dev/null
  source "$ROOT/scripts/eq_ephemeral_pg.sh"
  eq_ephemeral_pg_start "$profile" "edgequake-perf"
  assert_extensions "$EQ_EPHEMERAL_PG_CONTAINER" "$EQ_AGE_MIN" "$EQ_PGVECTOR_MIN"

  echo "  applying sqlx migrations (eq_next_node_id / native graph helpers)…"
  if ! command -v sqlx >/dev/null 2>&1; then
    echo "ERROR: sqlx-cli not found; install with:"
    echo "  cargo install sqlx-cli --no-default-features --features postgres --locked"
    return 1
  fi
  eq_ephemeral_pg_migrate
  docker exec -e PGPASSWORD="$PGPASSWORD" "$EQ_EPHEMERAL_PG_CONTAINER" \
    psql -U edgequake -d edgequake -At -c \
    "SELECT proname FROM pg_proc WHERE proname IN ('eq_next_node_id','eq_next_edge_id') ORDER BY 1;" \
    | grep -q eq_next_node_id || {
      echo "ERROR: eq_next_node_id missing after migrate"
      return 1
    }

  echo "  DATABASE_URL=$DATABASE_URL"
  run_cargo_gates "$profile"
  echo "OK $profile"
}

case "$PROFILE_ARG" in
  all)
    for p in pg16 pg17 pg18; do run_profile "$p"; done
    echo ""
    echo "SPEC-062 cross-major 2× gate…"
    python3 "$ROOT/scripts/compare_eq_perf_jsonl.py" --cross-major \
      /tmp/eq-perf-pg16.jsonl /tmp/eq-perf-pg17.jsonl /tmp/eq-perf-pg18.jsonl
    ;;
  pg16|pg17|pg18)
    run_profile "$PROFILE_ARG"
    ;;
  *)
    echo "Usage: $0 [pg16|pg17|pg18|all]"
    exit 1
    ;;
esac

echo ""
echo "SPEC-061/062 data-access perf matrix complete."
