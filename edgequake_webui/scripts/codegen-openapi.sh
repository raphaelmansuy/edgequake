#!/usr/bin/env bash
# OAS-009: Generate TypeScript types from EdgeQuake OpenAPI spec.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OPENAPI_DIR="$ROOT/openapi"
SNAPSHOT="$OPENAPI_DIR/openapi.snapshot.json"
OUT="$OPENAPI_DIR/schema.d.ts"
URL="${OPENAPI_URL:-http://localhost:8080/api-docs/openapi.json}"

mkdir -p "$OPENAPI_DIR"

if [[ "${1:-}" == "--offline" ]]; then
  INPUT="$SNAPSHOT"
else
  echo "Fetching OpenAPI from $URL ..."
  curl -sf "$URL" -o "$SNAPSHOT"
  INPUT="$SNAPSHOT"
fi

echo "Generating TypeScript types -> $OUT"
bunx openapi-typescript "$INPUT" -o "$OUT"
echo "Done."
