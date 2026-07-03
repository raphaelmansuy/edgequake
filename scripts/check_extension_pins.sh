#!/usr/bin/env bash
# Verify Dockerfile.postgres* ARG defaults match extension-pins.sh (SPEC-042 DRY gate).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-pg16}"

verify_profile() {
  local profile=$1
  local dockerfile
  case "$profile" in
    pg16) dockerfile="$ROOT/edgequake/docker/Dockerfile.postgres" ;;
    pg17) dockerfile="$ROOT/edgequake/docker/Dockerfile.postgres.pg17" ;;
    pg18) dockerfile="$ROOT/edgequake/docker/Dockerfile.postgres.pg18" ;;
    *) echo "Unknown profile: $profile"; return 1 ;;
  esac

  EQ_POSTGRES_PROFILE=$profile
  export EQ_POSTGRES_PROFILE
  # shellcheck source=/dev/null
  source "$ROOT/edgequake/docker/extension-pins.sh"

  local fail=0
  check() {
    local label=$1 pattern=$2
    if ! grep -q "$pattern" "$dockerfile"; then
      echo "FAIL $label: expected in $(basename "$dockerfile")"
      fail=1
    else
      echo "OK   $label ($profile)"
    fi
  }

  check "PGVECTOR_VERSION=${EQ_PGVECTOR_VERSION}" "PGVECTOR_VERSION=${EQ_PGVECTOR_VERSION}"
  check "PGVECTOR default_version='${EQ_PGVECTOR_MIN}'" "default_version = '${EQ_PGVECTOR_MIN}'"
  check "AGE_GIT_REF=${EQ_AGE_GIT_REF}" "AGE_GIT_REF=${EQ_AGE_GIT_REF}"
  check "AGE default_version='${EQ_AGE_MIN}'" "default_version = '${EQ_AGE_MIN}'"
  [ "$fail" -eq 0 ] || return 1
  echo "✓ Extension pins consistent ($profile ↔ $(basename "$dockerfile"))"
}

case "$PROFILE" in
  all)
    verify_profile pg16 && verify_profile pg17 && verify_profile pg18
    ;;
  pg16|pg17|pg18)
    verify_profile "$PROFILE"
    ;;
  *)
    echo "Usage: $0 [pg16|pg17|pg18|all]"; exit 1 ;;
esac
