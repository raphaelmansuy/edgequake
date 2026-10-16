#!/usr/bin/env bash
# scripts/check_epoch_coverage.sh — every published vX.Y.Z tag must map to an
# epoch in scripts/spec150/epochs.toml (same numbered-migration set hash).
# HEAD may match an epoch, or the [unreleased] pin, until the next tag exists.
#
# Usage:
#   ./scripts/check_epoch_coverage.sh
#   ./scripts/check_epoch_coverage.sh --json
#
# Exit 0 when every product tag is covered; exit 1 with a clear list otherwise.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

EPOCHS_TOML="$REPO_ROOT/scripts/spec150/epochs.toml"
JSON=0
[[ "${1:-}" == "--json" ]] && JSON=1

need() { command -v "$1" >/dev/null || { echo "missing $1" >&2; exit 1; }; }
need git
need python3

python3 - "$EPOCHS_TOML" "$JSON" <<'PY'
import hashlib, re, subprocess, sys, json
from collections import OrderedDict

epochs_path, as_json = sys.argv[1], sys.argv[2] == "1"
pat = re.compile(r"^v\d+\.\d+\.\d+$")

def mig_hash(ref: str):
    try:
        out = subprocess.check_output(
            ["git", "ls-tree", "-r", ref, "--", "edgequake/migrations"],
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except subprocess.CalledProcessError:
        return None
    entries = []
    maxv = 0
    for line in out.splitlines():
        meta, path = line.split("\t", 1)
        name = path.rsplit("/", 1)[-1]
        if not re.match(r"^\d{3}_.+\.sql$", name):
            continue
        blob = meta.split()[2]
        m = re.match(r"^(\d+)_", name)
        if m:
            maxv = max(maxv, int(m.group(1)))
        entries.append((name, blob))
    if not entries:
        return None
    entries.sort()
    h = hashlib.sha256()
    for name, blob in entries:
        h.update(name.encode())
        h.update(b"\0")
        h.update(blob.encode())
        h.update(b"\0")
    return h.hexdigest()[:16], maxv, len(entries)

def parse_unreleased(raw: str):
    """Working-tree pin. Not a product tag and not an epoch-matrix replay."""
    match = re.search(r"(?m)^\[unreleased\]\s*$", raw)
    if not match:
        return None
    body = raw[match.end():]
    stop = re.search(r"(?m)^\[\[|^\[", body)
    if stop:
        body = body[: stop.start()]
    fields = {}
    for line in body.splitlines():
        line = line.split("#", 1)[0].strip()
        if not line or "=" not in line:
            continue
        key, value = line.split("=", 1)
        fields[key.strip()] = value.strip().strip('"')
    return fields or None

# Parse epoch tags from epochs.toml
epoch_tags = []
raw = open(epochs_path).read()
unreleased = parse_unreleased(raw)
text = raw.split("[[epoch]]")
for block in text[1:]:
    for line in block.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("tag"):
            _, v = line.split("=", 1)
            epoch_tags.append(v.strip().strip('"'))
            break

epoch_hashes = {}
missing_epochs = []
for tag in epoch_tags:
    r = mig_hash(tag)
    if r is None:
        missing_epochs.append(tag)
        continue
    epoch_hashes[r[0]] = tag

tags = subprocess.check_output(["git", "tag", "-l", "v*"], text=True).split()
tags = sorted([t for t in tags if pat.match(t)], key=lambda t: tuple(map(int, t[1:].split("."))))

uncovered = []
covered = []
for t in tags:
    r = mig_hash(t)
    if r is None:
        uncovered.append({"tag": t, "reason": "no_migrations_tree"})
        continue
    h, maxv, n = r
    if h in epoch_hashes:
        covered.append({"tag": t, "hash": h, "max": maxv, "n": n, "epoch": epoch_hashes[h]})
    else:
        uncovered.append({"tag": t, "hash": h, "max": maxv, "n": n, "reason": "no_matching_epoch"})

# HEAD matches a published epoch, or the [unreleased] pin for a train
# that does not have a product tag yet.
head = mig_hash("HEAD")
pin_mismatch = None
head_pin_ok = False
if head is not None and unreleased and unreleased.get("hash") == head[0]:
    problems = []
    if "max" in unreleased and int(unreleased["max"]) != head[1]:
        problems.append(f"max pin {unreleased['max']} != HEAD {head[1]}")
    if "files" in unreleased and int(unreleased["files"]) != head[2]:
        problems.append(f"files pin {unreleased['files']} != HEAD {head[2]}")
    if problems:
        pin_mismatch = "; ".join(problems)
    else:
        head_pin_ok = True
elif head is not None and unreleased and unreleased.get("hash") != head[0]:
    pin_mismatch = (
        f"unreleased pin {unreleased.get('hash')} != HEAD {head[0]} "
        f"(max={head[1]} files={head[2]})"
    )
head_ok = head is not None and (head[0] in epoch_hashes or head_pin_ok)

result = {
    "epoch_tags": epoch_tags,
    "product_tags": len(tags),
    "covered": len(covered),
    "uncovered": uncovered,
    "missing_epoch_refs": missing_epochs,
    "head": {
        "hash": None if head is None else head[0],
        "max": None if head is None else head[1],
        "n": None if head is None else head[2],
        "covered": head_ok,
        "epoch": (
            None
            if head is None or not head_ok
            else epoch_hashes.get(head[0], "unreleased" if head_pin_ok else None)
        ),
        "pin_mismatch": pin_mismatch,
    },
}

if as_json:
    print(json.dumps(result, indent=2))
else:
    print("SPEC-150 epoch coverage check")
    print(f"  epochs.toml entries : {len(epoch_tags)}")
    print(f"  product tags (vX.Y.Z): {len(tags)}")
    print(f"  covered             : {len(covered)}")
    print(f"  uncovered           : {len(uncovered)}")
    if head is not None:
        if head[0] in epoch_hashes:
            status = "OK"
        elif head_pin_ok:
            status = "OK unreleased"
        else:
            status = "MISSING"
        print(f"  HEAD schema set     : max={head[1]} n={head[2]} hash={head[0]} [{status}]")
        if pin_mismatch and head[0] not in epoch_hashes:
            print(f"  unreleased pin      : {pin_mismatch}")
    if missing_epochs:
        print("  epoch tags not in git:")
        for t in missing_epochs:
            print(f"    - {t}")
    if uncovered:
        print("  UNCOVERED tags (add a matching [[epoch]] to epochs.toml):")
        for u in uncovered:
            if "hash" in u:
                print(f"    - {u['tag']}  max={u['max']} n={u['n']} hash={u['hash']}")
            else:
                print(f"    - {u['tag']}  ({u['reason']})")
        sys.exit(1)
    if missing_epochs or not head_ok:
        sys.exit(1)
    print("PASS: every published product tag maps to an epoch; HEAD is covered.")

sys.exit(0 if not uncovered and not missing_epochs and head_ok else 1)
PY
