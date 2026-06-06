#!/usr/bin/env bash
# SPEC-006: thin wrapper — delegates to canonical edgequake script.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec "$ROOT/edgequake/scripts/migrations/apply_038.sh" "$@"
