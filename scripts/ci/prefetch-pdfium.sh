#!/usr/bin/env bash
# pdfium-auto 0.3.1 downloads chromium%2F7961. Both that encoded tag and the
# slash-form HTML download URL return HTTP 504 often enough to abort the
# build. The release-asset API redirects to release-assets.githubusercontent.com
# and that copy downloads. Print the extracted library path so the caller can
# set PDFIUM_BUNDLE_LIB. Asset ids are the chromium/7961 release.
set -euo pipefail

version=7961
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    asset=483237356
    member=lib/libpdfium.so
    name=libpdfium.so
    ;;
  Linux-aarch64)
    asset=483237422
    member=lib/libpdfium.so
    name=libpdfium.so
    ;;
  Darwin-arm64)
    asset=483237332
    member=lib/libpdfium.dylib
    name=libpdfium.dylib
    ;;
  Darwin-x86_64)
    asset=483237311
    member=lib/libpdfium.dylib
    name=libpdfium.dylib
    ;;
  *)
    echo "prefetch-pdfium: unsupported host $(uname -s)-$(uname -m)" >&2
    exit 1
    ;;
esac

dest="${PDFIUM_PREFETCH_DIR:-${HOME}/.cache/edgequake-pdfium/${version}}"
lib="${dest}/${name}"
if [[ ! -s "$lib" ]]; then
  mkdir -p "$dest"
  tmp="$(mktemp)"
  url="https://api.github.com/repos/bblanchon/pdfium-binaries/releases/assets/${asset}"
  auth=()
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    auth=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
  fi
  curl -fL --retry 5 --retry-all-errors --retry-delay 2 \
    -H "Accept: application/octet-stream" "${auth[@]}" \
    -o "$tmp" "$url"
  tar -xzf "$tmp" -C "$dest" "$member"
  mv "${dest}/${member}" "$lib"
  rm -f "$tmp"
  rmdir "${dest}/lib" 2>/dev/null || true
fi
printf '%s\n' "$lib"
