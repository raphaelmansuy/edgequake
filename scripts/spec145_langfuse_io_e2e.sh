#!/usr/bin/env bash
# SPEC-145: Complete Langfuse observation I/O proof.
# CI: unit + InMemory (no keys). Live: LANGFUSE_SPEC145_E2E=1 + base/keys
# (make spec145-langfuse-e2e starts Langfuse 3.225.5 and injects keys).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BACKEND_DIR="${ROOT}/edgequake"

echo "→ SPEC-145 proof (InMemory + io_policy + stream contract)"
cd "${BACKEND_DIR}"
cargo test -p edgequake-observability --lib io_policy --quiet
cargo test -p edgequake-observability --lib inmemory_spec145 --quiet
cargo test -p edgequake-observability --lib rag_span --quiet
cargo test -p edgequake-query --lib spec124_stream_genai --quiet
echo "✓ SPEC-145 CI proof passed"

if [ "${LANGFUSE_SPEC145_E2E:-0}" = "1" ]; then
  echo "→ SPEC-145 live Langfuse Complete I/O (LANGFUSE_SPEC145_E2E=1)"
  # shellcheck source=/dev/null
  source "${ROOT}/scripts/langfuse_e2e_common.sh"
  BASE="${LANGFUSE_BASE_URL:-${LANGFUSE_OTLP_E2E_BASE:-}}"
  PK="${LANGFUSE_PUBLIC_KEY:-}"
  SK="${LANGFUSE_SECRET_KEY:-}"
  if [ -z "${BASE}" ] || [ -z "${PK}" ] || [ -z "${SK}" ]; then
    echo "✗ LANGFUSE_SPEC145_E2E=1 requires LANGFUSE_BASE_URL + PUBLIC/SECRET keys" >&2
    exit 1
  fi
  code="$(curl -sS -o /tmp/eq-spec145-health.json -w '%{http_code}' "${BASE%/}/api/public/health" || true)"
  if [ "${code}" != "200" ]; then
    echo "✗ Langfuse not healthy at ${BASE} (HTTP ${code}) — start with make langfuse-3.225-up" >&2
    exit 1
  fi
  langfuse_assert_otlp_exists "${BASE}" "${PK}" "${SK}"
  export LANGFUSE_SPEC145_E2E=1
  export LANGFUSE_BASE_URL="${BASE}"
  export LANGFUSE_OTLP_E2E_BASE="${BASE}"
  export LANGFUSE_PUBLIC_KEY="${PK}"
  export LANGFUSE_SECRET_KEY="${SK}"
  cargo test -p edgequake-observability --lib live_spec145_complete_io_roundtrip -- --nocapture
  echo "✓ SPEC-145 live Langfuse I/O passed"
else
  echo "· skip live Langfuse (make spec145-langfuse-e2e to enable)"
fi
