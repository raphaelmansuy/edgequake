#!/usr/bin/env bash
# Pre-release quality gates — fmt, per-crate clippy, lib tests, SPEC-006 + SPEC-018 proofs.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EQ="$ROOT/edgequake"
WEBUI="$ROOT/edgequake_webui"

CRATES=(
  edgequake-api
  edgequake-audit
  edgequake-auth
  edgequake-core
  edgequake-observability
  edgequake-pdf
  edgequake-pipeline
  edgequake-query
  edgequake-rate-limiter
  edgequake-storage
  edgequake-tasks
)

echo "== rustfmt =="
(cd "$EQ" && cargo fmt --all -- --check)

echo "== workspace clippy =="
(cd "$EQ" && cargo clippy --workspace --lib -- -D warnings)

echo "== per-crate clippy =="
for crate in "${CRATES[@]}"; do
  echo "→ clippy -p $crate"
  FEATURES=()
  case "$crate" in
    edgequake-api|edgequake-core|edgequake-storage|edgequake-tasks)
      FEATURES=(--features postgres)
      ;;
  esac
  (cd "$EQ" && cargo clippy -p "$crate" --lib "${FEATURES[@]}" -- -D warnings)
done

echo "== workspace lib tests =="
if [[ "${RELEASE_SKIP_LIB_TESTS:-}" == "1" ]]; then
  echo "skipped (RELEASE_SKIP_LIB_TESTS=1 — full suite runs on main CI)"
else
  (cd "$EQ" && cargo test --workspace --lib --no-fail-fast)
fi

echo "== SPEC-006 resource-proof =="
(cd "$ROOT" && make resource-proof --no-print-directory)

echo "== SPEC-018 observability-proof =="
chmod +x "$ROOT/specs/018-observability/e2e/run_observability_proof.sh"
"$ROOT/specs/018-observability/e2e/run_observability_proof.sh"

echo "== WebUI typecheck (src only; e2e via Playwright) =="
(cd "$WEBUI" && bunx tsc --noEmit -p tsconfig.release.json)

echo "== WebUI unit tests (observability + runtime-config) =="
(cd "$WEBUI" && bun test src/lib/api/__tests__/observability-client.test.ts src/lib/__tests__/runtime-config.test.ts)

echo "✓ release gates passed"
