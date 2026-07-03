#!/usr/bin/env bash
# SPEC-042 — Run all battle tests (pins + version matrix + Phase E + #275 HNSW guard).
#
# Usage:
#   ./specs/042-update-age-pgvector/e2e/run_all_battle_tests.sh
#
# Optional env:
#   SKIP_IMAGE_BUILD=1  — do not rebuild postgres images
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
E2E="$ROOT/specs/042-update-age-pgvector/e2e"

echo "== SPEC-042 full battle test suite =="
echo ""

if [ "${SKIP_IMAGE_BUILD:-0}" != "1" ]; then
  echo "→ Building postgres images (pg16 + pg17 + pg18)..."
  make -C "$ROOT" postgres-image-build postgres-image-build-pg17 postgres-image-build-pg18 --no-print-directory
  echo ""
fi

echo "→ [1/4] Extension pin SSOT (check_extension_pins.sh all)..."
chmod +x "$ROOT/scripts/check_extension_pins.sh"
"$ROOT/scripts/check_extension_pins.sh" all
echo ""

echo "→ [2/4] Version feature battle test (pg16 + pg17 + pg18)..."
chmod +x "$E2E/run_version_feature_battle_test.sh"
"$E2E/run_version_feature_battle_test.sh" all
echo ""

echo "→ [3/4] Phase E battle test (pg17 + pg18)..."
chmod +x "$E2E/run_phase_e_battle_test.sh"
"$E2E/run_phase_e_battle_test.sh" all
echo ""

echo "→ [4/4] HNSW dimension guard battle test #275 (pg16 + pg17 + pg18)..."
chmod +x "$E2E/run_hnsw_dimension_battle_test.sh"
"$E2E/run_hnsw_dimension_battle_test.sh" all
echo ""

echo "→ [5/5] Rust unit probes (AnnIndexPolicy + M071 checksum)..."
cd "$ROOT/edgequake"
cargo test -p edgequake-storage --features postgres --lib ann_index_policy_tests --quiet
cargo test -p edgequake-api --features postgres --lib pre_and_fixed_checksums --quiet
echo ""

echo "✓ SPEC-042 FULL BATTLE TEST SUITE PASSED"
