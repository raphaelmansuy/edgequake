#!/usr/bin/env bash
# pdfium-auto 0.3.1 downloads chromium%2F7961. GitHub answers that URL with
# HTTP 502/503/504 often enough to abort CI. The chromium/7961 linux archives
# are vendored in-tree. Print the extracted library path so the caller can
# set PDFIUM_BUNDLE_LIB. Other hosts fall back to the public release-asset API
# without a workflow token (that token is rejected with HTTP 403).
set -euo pipefail

version=7961
repo="$(cd "$(dirname "$0")/../.." && pwd)"
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)
    archive=pdfium-linux-x64.tgz
    sha=019665c8877d46fe65f625f80fd714ab07aac68554b0636acf2a2adf9288adb2
    member=lib/libpdfium.so
    name=libpdfium.so
    asset=483237356
    ;;
  Linux-aarch64)
    archive=pdfium-linux-arm64.tgz
    sha=974107999784a438149605024475d42d80dd306799d90e1af5f6fa63f976455f
    member=lib/libpdfium.so
    name=libpdfium.so
    asset=483237422
    ;;
  Darwin-arm64)
    archive=
    sha=
    member=lib/libpdfium.dylib
    name=libpdfium.dylib
    asset=483237332
    ;;
  Darwin-x86_64)
    archive=
    sha=
    member=lib/libpdfium.dylib
    name=libpdfium.dylib
    asset=483237311
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
  vendor="${repo}/edgequake/third_party/pdfium/${archive}"
  if [[ -n "$archive" && -s "$vendor" ]]; then
    actual="$(sha256sum "$vendor" | awk '{print $1}')"
    if [[ "$actual" != "$sha" ]]; then
      echo "prefetch-pdfium: ${archive} sha256 ${actual} != ${sha}" >&2
      exit 1
    fi
    tar -xzf "$vendor" -C "$dest" "$member"
  else
    tmp="$(mktemp)"
    curl -fL --retry 5 --retry-all-errors --retry-delay 2 \
      -H "Accept: application/octet-stream" \
      -o "$tmp" \
      "https://api.github.com/repos/bblanchon/pdfium-binaries/releases/assets/${asset}"
    tar -xzf "$tmp" -C "$dest" "$member"
    rm -f "$tmp"
  fi
  mv "${dest}/${member}" "$lib"
  rmdir "${dest}/lib" 2>/dev/null || true
fi
printf '%s\n' "$lib"
