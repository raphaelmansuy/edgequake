#!/usr/bin/env bash
# oven-sh/setup-bun downloads the HTML release URL. GitHub answers that with
# HTTP 504 often enough to fail the job after two tries. The release-asset
# API redirects to a copy that downloads. Print the directory that contains
# the bun binary so the caller can put it on PATH.
set -euo pipefail

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) asset_name=bun-linux-x64.zip ;;
  Linux-aarch64) asset_name=bun-linux-aarch64.zip ;;
  Darwin-arm64) asset_name=bun-darwin-aarch64.zip ;;
  Darwin-x86_64) asset_name=bun-darwin-x64.zip ;;
  *)
    echo "install-bun: unsupported host $(uname -s)-$(uname -m)" >&2
    exit 1
    ;;
esac

dest="${BUN_PREFETCH_DIR:-${HOME}/.cache/edgequake-bun}"
bin="${dest}/bin"
if [[ ! -x "${bin}/bun" ]]; then
  mkdir -p "$bin"
  auth=()
  if [[ -n "${GITHUB_TOKEN:-}" ]]; then
    auth=(-H "Authorization: Bearer ${GITHUB_TOKEN}")
  fi
  meta="$(mktemp)"
  curl -fsSL --retry 5 --retry-all-errors --retry-delay 2 \
    -H "Accept: application/vnd.github+json" \
    -H "User-Agent: edgequake-ci" \
    "${auth[@]}" \
    -o "$meta" \
    "https://api.github.com/repos/oven-sh/bun/releases/latest"
  asset_id="$(python3 -c '
import json, sys
release = json.load(open(sys.argv[1]))
want = sys.argv[2]
for asset in release.get("assets", []):
    if asset["name"] == want:
        print(asset["id"])
        raise SystemExit(0)
raise SystemExit("install-bun: %s missing from %s" % (want, release.get("tag_name")))
' "$meta" "$asset_name")"
  rm -f "$meta"
  tmp="$(mktemp)"
  curl -fL --retry 5 --retry-all-errors --retry-delay 2 \
    -H "Accept: application/octet-stream" \
    -H "User-Agent: edgequake-ci" \
    "${auth[@]}" \
    -o "$tmp" \
    "https://api.github.com/repos/oven-sh/bun/releases/assets/${asset_id}"
  work="$(mktemp -d)"
  unzip -q "$tmp" -d "$work"
  rm -f "$tmp"
  found="$(find "$work" -type f -name bun -print -quit)"
  if [[ -z "$found" ]]; then
    echo "install-bun: archive has no bun binary" >&2
    exit 1
  fi
  mv "$found" "${bin}/bun"
  chmod +x "${bin}/bun"
  rm -rf "$work"
fi
# release_gates.sh calls bunx. The official zip ships that name as a symlink
# to bun, and bun dispatches on argv0. A cache hit that only copied bun
# still needs the name.
if [[ ! -e "${bin}/bunx" ]]; then
  ln -s bun "${bin}/bunx"
fi
printf '%s\n' "$bin"
