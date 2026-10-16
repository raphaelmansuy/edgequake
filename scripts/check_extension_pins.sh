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
    pg18-vectorscale) dockerfile="$ROOT/edgequake/docker/Dockerfile.postgres.pg18-vectorscale" ;;
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
  if [ -n "${EQ_AGE_MIN:-}" ]; then
    check "AGE_GIT_REF=${EQ_AGE_GIT_REF}" "AGE_GIT_REF=${EQ_AGE_GIT_REF}"
    check "AGE default_version='${EQ_AGE_MIN}'" "default_version = '${EQ_AGE_MIN}'"
  elif grep -q "org.edgequake.age.version" "$dockerfile"; then
    echo "FAIL product image $(basename "$dockerfile") must not label an AGE version"
    fail=1
  else
    echo "OK   no Apache AGE ($profile)"
  fi
  if [ -n "${EQ_PGVECTORSCALE_MIN:-}" ]; then
    check "PGVECTORSCALE_VERSION=${EQ_PGVECTORSCALE_VERSION}" "PGVECTORSCALE_VERSION=${EQ_PGVECTORSCALE_VERSION}"
    check "vectorscale default_version='${EQ_PGVECTORSCALE_MIN}'" "default_version = '${EQ_PGVECTORSCALE_MIN}'"
  fi
  [ "$fail" -eq 0 ] || return 1
  echo "✓ Extension pins consistent ($profile ↔ $(basename "$dockerfile"))"
}

verify_docs_pgvector_pin() {
  EQ_POSTGRES_PROFILE=pg18
  export EQ_POSTGRES_PROFILE
  # shellcheck source=/dev/null
  source "$ROOT/edgequake/docker/extension-pins.sh"
  local pin="${EQ_PGVECTOR_MIN}"
  local fail=0
  local stale="0.8.3"
  for f in "$ROOT/Makefile" "$ROOT/edgequake/docker/README.md"; do
    if grep -E "pgvector[^0-9]*${stale}|${stale}[^0-9]*pgvector|\`pgvector\` ${stale}" "$f" >/dev/null 2>&1; then
      echo "FAIL stale pgvector ${stale} in ${f#"$ROOT"/} (SSOT pin is ${pin})"
      fail=1
    fi
  done
  if ! grep -q "pgvector ${pin}" "$ROOT/Makefile"; then
    echo "FAIL Makefile missing pgvector ${pin} help text (SSOT pin)"
    fail=1
  fi
  if ! grep -q "pgvector.*${pin}" "$ROOT/edgequake/docker/README.md"; then
    echo "FAIL docker/README.md missing pgvector ${pin} (SSOT pin)"
    fail=1
  fi
  [ "$fail" -eq 0 ] || return 1
  echo "✓ Docs/Makefile pgvector pin matches extension-pins.sh (${pin})"
}

case "$PROFILE" in
  all)
    verify_profile pg16
    verify_profile pg17
    verify_profile pg18
    verify_profile pg18-vectorscale
    verify_docs_pgvector_pin
    ;;
  pg16|pg17|pg18|pg18-vectorscale)
    verify_profile "$PROFILE"
    ;;
  docs)
    verify_docs_pgvector_pin
    ;;
  *)
    echo "Usage: $0 [pg16|pg17|pg18|pg18-vectorscale|all|docs]"; exit 1 ;;
esac
