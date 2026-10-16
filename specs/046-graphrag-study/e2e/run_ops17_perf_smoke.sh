#!/usr/bin/env bash
# SPEC-046 OPS-P2.17 — PG16/17/18 smoke harness (pins + optional battle test).
#
# Non-flaky by default: validates extension-pins.sh SSOT for each profile without
# requiring a running Postgres. Pass --battle to run SPEC-042 battle tests
# (needs Docker images).
#
# Usage:
#   ./specs/046-graphrag-study/e2e/run_ops17_perf_smoke.sh
#   ./specs/046-graphrag-study/e2e/run_ops17_perf_smoke.sh --battle
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
PINS="$ROOT/edgequake/docker/extension-pins.sh"
BATTLE=0

for arg in "$@"; do
  case "$arg" in
    --battle) BATTLE=1 ;;
    -h|--help)
      echo "Usage: $0 [--battle]"
      exit 0
      ;;
  esac
done

if [[ ! -f "$PINS" ]]; then
  echo "FAIL: extension-pins.sh not found at $PINS" >&2
  exit 1
fi

echo "== OPS-17: validate extension pins for pg16/pg17/pg18 =="
for profile in pg16 pg17 pg18; do
  # shellcheck disable=SC1090
  EQ_POSTGRES_PROFILE="$profile" source "$PINS"
  : "${EQ_POSTGRES_MAJOR:?}"
  : "${EQ_PGVECTOR_MIN:?}"
  if [ -n "${EQ_AGE_MIN:-}" ]; then
    echo "FAIL $profile product image must not pin Apache AGE (got ${EQ_AGE_MIN})" >&2
    exit 1
  fi
  echo "  OK $profile → PG${EQ_POSTGRES_MAJOR} pgvector>=${EQ_PGVECTOR_MIN} AGE absent"
done

if [[ "$BATTLE" -eq 1 ]]; then
  echo "== OPS-17: SPEC-042 battle suite (all profiles) =="
  if [[ -x "$ROOT/specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh" ]]; then
    "$ROOT/specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh"
  else
    echo "FAIL: battle orchestrator missing" >&2
    exit 1
  fi
else
  echo "== OPS-17: pin smoke only (pass --battle for Docker battle tests) =="
fi

echo "OPS-17 smoke OK"
