#!/usr/bin/env bash
# utoipa-swagger-ui 9 downloads this zip once, with no retry. A dropped
# connection (curl exit 56) aborts the release build. The github.com archive
# URL also returns HTTP 504; codeload serves the same tag zip. Print the path
# so the caller can set SWAGGER_UI_DOWNLOAD_URL=file://<path>.
set -euo pipefail

version=v5.17.14
sha=481244d0812097b11fbaeef79f71d942b171617f9c9f9514e63acbe13e71ccdc
repo="$(cd "$(dirname "$0")/../.." && pwd)"
vendor="${repo}/edgequake/third_party/swagger-ui/swagger-ui-${version}.zip"
if [[ -s "$vendor" ]]; then
  actual="$(sha256sum "$vendor" | awk '{print $1}')"
  if [[ "$actual" != "$sha" ]]; then
    echo "prefetch-swagger-ui: sha256 ${actual} != ${sha}" >&2
    exit 1
  fi
  printf '%s\n' "$vendor"
  exit 0
fi
dest="${SWAGGER_UI_PREFETCH_DIR:-${HOME}/.cache/edgequake-swagger-ui}"
zip="${dest}/swagger-ui-${version}.zip"
if [[ ! -s "$zip" ]]; then
  mkdir -p "$dest"
  tmp="$(mktemp)"
  # The github.com archive URL returns HTTP 504. codeload serves the same zip.
  curl -fL --retry 5 --retry-all-errors --retry-delay 2 \
    -o "$tmp" \
    "https://codeload.github.com/swagger-api/swagger-ui/zip/refs/tags/${version}"
  size="$(wc -c < "$tmp")"
  if [[ "$size" -lt 100000 ]]; then
    echo "prefetch-swagger-ui: download is only ${size} bytes" >&2
    exit 1
  fi
  mv "$tmp" "$zip"
fi
printf '%s\n' "$zip"
