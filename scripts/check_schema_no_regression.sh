#!/usr/bin/env bash
# scripts/check_schema_no_regression.sh — HEAD must never serve an older schema
# than a published release.
#
# WHY this exists: `check_schema_train_docs.sh` only verifies *internal*
# consistency — that `compat_serve_max` matches the highest migration file
# present, and that the docs mention that number. All of those can agree on an
# older value and still pass. That is exactly what happened between v0.40.7 and
# v0.40.12: migrations 228–231 were tagged from a side branch, `edgequake-main`
# stayed at 227, and every internal check was green while a release cut from main
# would have declared schema 227 against databases already serving 231.
#
# The rule enforced here is one-directional and simple:
#
#   HEAD's compat_serve_max  >=  compat_serve_max of every published vX.Y.Z tag
#
# A published tag is a fact about databases in the field. Code that cannot read
# a schema that has already shipped is a downgrade, however green the build is.
#
# Usage:
#   ./scripts/check_schema_no_regression.sh          # gate
#   ./scripts/check_schema_no_regression.sh --json   # machine-readable
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

MANIFEST="$ROOT/edgequake/migrations/manifest.toml"
JSON=0
[[ "${1:-}" == "--json" ]] && JSON=1

need() { command -v "$1" >/dev/null || { echo "missing $1" >&2; exit 1; }; }
need git
need python3

python3 - "$MANIFEST" "$JSON" <<'PY'
import json
import re
import subprocess
import sys

manifest_path, as_json = sys.argv[1], sys.argv[2] == "1"


def compat_max(ref: str):
    """`compat_serve_max` from manifest.toml at `ref`, or None when absent."""
    try:
        raw = subprocess.check_output(
            ["git", "show", f"{ref}:edgequake/migrations/manifest.toml"],
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError:
        return None
    match = re.search(r"(?m)^compat_serve_max\s*=\s*(\d+)", raw)
    return int(match.group(1)) if match else None


head_raw = open(manifest_path).read()
head_match = re.search(r"(?m)^compat_serve_max\s*=\s*(\d+)", head_raw)
if not head_match:
    print("ERROR: cannot parse compat_serve_max from HEAD manifest.toml", file=sys.stderr)
    sys.exit(1)
head_max = int(head_match.group(1))

tag_pattern = re.compile(r"^v\d+\.\d+\.\d+$")
tags = subprocess.check_output(["git", "tag", "-l", "v*"], text=True).split()
tags = sorted(
    (t for t in tags if tag_pattern.match(t)),
    key=lambda t: tuple(int(p) for p in t[1:].split(".")),
)

# Only released tags constrain us. An un-pushed local tag is not a database in
# the field; remote tags are. `git tag -l` sees both, so filter by whether the
# tag exists on a remote.
remote_tags = set()
try:
    out = subprocess.check_output(
        ["git", "ls-remote", "--tags", "--refs", "origin"], text=True, stderr=subprocess.DEVNULL
    )
    for line in out.splitlines():
        name = line.rsplit("refs/tags/", 1)[-1]
        if tag_pattern.match(name):
            remote_tags.add(name)
except Exception:
    # No network / no origin: fall back to every local tag rather than skipping
    # the gate. A false positive is a conversation; a false negative is a bug.
    remote_tags = set(tags)

published = [t for t in tags if t in remote_tags]
regressions = []
for tag in published:
    tag_max = compat_max(tag)
    if tag_max is None:
        continue  # tag predates manifest.toml
    if tag_max > head_max:
        regressions.append({"tag": tag, "tag_max": tag_max, "head_max": head_max})

newest = None
for tag in reversed(published):
    value = compat_max(tag)
    if value is not None:
        newest = {"tag": tag, "max": value}
        break

result = {
    "head_max": head_max,
    "published_tags": len(published),
    "newest_published": newest,
    "regressions": regressions,
}

if as_json:
    print(json.dumps(result, indent=2))
else:
    print("Schema regression check (HEAD vs published tags)")
    print(f"  HEAD compat_serve_max    : {head_max}")
    print(f"  published vX.Y.Z tags    : {len(published)}")
    if newest:
        print(f"  newest published schema  : {newest['max']} ({newest['tag']})")
    if regressions:
        print("")
        print("  REGRESSION: HEAD cannot serve a schema that has already shipped:")
        for r in regressions:
            print(f"    - {r['tag']} serves {r['tag_max']}, HEAD declares {r['head_max']}")
        print("")
        print("  Fix: merge the tag's line into this branch, or raise")
        print("       edgequake/migrations/manifest.toml compat_serve_max and add")
        print("       the missing migrations. Do not ship a release from this tree.")

if regressions:
    sys.exit(1)

print("PASS: HEAD serves every schema a published release has shipped.")
sys.exit(0)
PY
