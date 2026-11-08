#!/usr/bin/env bash
# Append PDFIUM_BUNDLE_LIB and SWAGGER_UI_DOWNLOAD_URL to a GitHub Actions
# env file. A failed download fails this script. Do not wrap the calls in
# echo "$(...)" — that reports success and writes an empty value.
set -euo pipefail

dest="${1:?env file}"
here="$(cd "$(dirname "$0")" && pwd)"
pdfium="$(bash "$here/prefetch-pdfium.sh")"
swagger="$(bash "$here/prefetch-swagger-ui.sh")"
{
  printf 'PDFIUM_BUNDLE_LIB=%s\n' "$pdfium"
  printf 'SWAGGER_UI_DOWNLOAD_URL=file://%s\n' "$swagger"
} >> "$dest"
