#!/usr/bin/env bash
# SPEC-006: G-006 — verify ResourceBudgetConfig defaults match catalog.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/edgequake"

cargo test -p edgequake-core resource_budget_defaults_match_catalog --quiet
echo "✓ SPEC-006: ResourceBudgetConfig defaults match catalog"
