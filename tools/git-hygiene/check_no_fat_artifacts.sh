#!/usr/bin/env bash
# SPEC-097 / GH-351 — fail if regenerable fat bench artifacts or oversized blobs are tracked.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

MAX_BLOB_BYTES=$((50 * 1024 * 1024)) # 50 MiB (GitHub warning threshold)
FAIL=0

echo "==> SPEC-097 git-hygiene: fat artifact globs"

mapfile -t fat_hits < <(
  {
    git ls-files -- \
      'specs/001-benchmark/e2e/artifacts/**/predictions_*.json' \
      'specs/001-benchmark/e2e/artifacts/**/eval_*.json' \
      'specs/001-benchmark/e2e/artifacts/**/eval_*.raw.json' \
      'specs/001-benchmark/e2e/artifacts/**/logs/progress.jsonl' \
      'specs/001-benchmark/e2e/artifacts/history/**/logs/**' \
      2>/dev/null || true
    git ls-files 'specs/001-benchmark/e2e/artifacts' \
      | grep -E '(^|/)(predictions_[^/]+\.json|eval_[^/]+\.json)$' || true
    git ls-files 'specs/001-benchmark/e2e/artifacts/history' \
      | grep -E '/logs/' || true
  } | sort -u
)

if ((${#fat_hits[@]} > 0)) && [[ -n "${fat_hits[0]:-}" ]]; then
  echo "ERROR: tracked fat regenerable artifacts (SPEC-097 / LAW-G3):" >&2
  printf '  %s\n' "${fat_hits[@]}" >&2
  FAIL=1
else
  echo "OK: no fat bench001 artifacts tracked"
fi

echo "==> SPEC-097 git-hygiene: index blobs > 50 MiB"

# Build hash→path map from index, then batch-check sizes (fast).
tmp_map="$(mktemp)"
tmp_sizes="$(mktemp)"
trap 'rm -f "$tmp_map" "$tmp_sizes"' EXIT

git ls-files -s | awk '{print $2 "\t" $4}' >"$tmp_map"
awk -F'\t' '{print $1}' "$tmp_map" | git cat-file --batch-check='%(objectname) %(objectsize)' >"$tmp_sizes"

oversized=()
while read -r hash size; do
  [[ "$size" =~ ^[0-9]+$ ]] || continue
  if (( size > MAX_BLOB_BYTES )); then
    path="$(awk -F'\t' -v h="$hash" '$1==h {print $2; exit}' "$tmp_map")"
    oversized+=("$size ${path:-$hash}")
  fi
done <"$tmp_sizes"

if ((${#oversized[@]} > 0)); then
  echo "ERROR: tracked tip blobs larger than 50 MiB:" >&2
  printf '  %s\n' "${oversized[@]}" >&2
  echo "Remove/gitignore them (SPEC-097). Large intentional fixtures need a SPEC exception." >&2
  FAIL=1
else
  echo "OK: no index blobs over 50 MiB"
fi

if ((FAIL != 0)); then
  echo "git-hygiene FAILED — see specs/097-git-history/" >&2
  exit 1
fi

echo "git-hygiene PASSED"
exit 0
