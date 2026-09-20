#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TREE_FILE="${TMPDIR:-/tmp}/edgequake-p3-tree.txt"

cd "$ROOT_DIR/edgequake"
cargo tree \
  -p edgequake-storage \
  --no-default-features \
  --features p3 \
  -e normal,build \
  | tee "$TREE_FILE"

if rg -n '(^|[[:space:]├└│])sqlx-postgres v|(^|[[:space:]├└│])postgres(-protocol|-types)? v' \
  "$TREE_FILE"; then
  echo "P3 feature graph unexpectedly contains a PostgreSQL driver" >&2
  exit 1
fi

echo "P3 feature graph is PostgreSQL-driver-free"
