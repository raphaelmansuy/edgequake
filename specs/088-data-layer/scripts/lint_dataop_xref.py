#!/usr/bin/env python3
"""Fail if dataop inventory, code constants, and docs disagree (SPEC-088)."""
from __future__ import annotations
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
inv = json.loads((ROOT / "docs/data-layer/00-inventory.json").read_text())
refs = {o["ref"] for o in inv}

# dataop.rs constants — only ALL_REF_IDS / pub const assignments, not test fixtures
dataop = (ROOT / "edgequake/crates/edgequake-storage/src/dataop.rs").read_text()
code_refs = set(
    re.findall(r'pub const DATA_[A-Z0-9_]+: &str = "(DATA-(?:PG|PGVEC|AGE)-[A-Z0-9-]+)"', dataop)
)

# docs mention
doc_text = ""
for p in (ROOT / "specs/088-data-layer").rglob("*.md"):
    doc_text += p.read_text(errors="replace") + "\n"
doc_refs = set(re.findall(r"DATA-(?:PG|PGVEC|AGE)-[A-Z0-9-]+", doc_text))

errors = []
if refs - code_refs:
    errors.append(f"In inventory but missing from dataop.rs: {sorted(refs - code_refs)[:10]}… ({len(refs-code_refs)})")
if code_refs - refs:
    # ALL_REF_IDS may only have inventory; constants same
    only_code = code_refs - refs
    # allow nothing extra
    if only_code:
        errors.append(f"In dataop.rs but not inventory: {sorted(only_code)[:10]}")
if refs - doc_refs:
    errors.append(f"In inventory but not in specs/088 docs: {len(refs - doc_refs)} missing")

# duplicate NNN
nnns = [o["ref"].split("-")[-1] for o in inv]
if len(nnns) != len(set(nnns)):
    errors.append("Duplicate sequence numbers in inventory")

# annotation catalog must cover every inventory ref
ann_path = ROOT / "edgequake/crates/edgequake-storage/src/dataop_annotations.rs"
if not ann_path.exists():
    errors.append("missing dataop_annotations.rs catalog")
else:
    ann_text = ann_path.read_text()
    for r in sorted(refs):
        if f'("{r}"' not in ann_text and f'("{r}",' not in ann_text:
            errors.append(f"Missing annotation catalog entry: {r}")
            break

# annotation presence for hot paths (inline in production sources)
hot = [
    "DATA-PGVEC-VECTORS-ANN-QUERY-001",
    "DATA-PGVEC-VECTORS-ANN-QUERY-FILTERED-002",
    "DATA-PGVEC-VECTORS-UPSERT-BATCH-004",
    "DATA-AGE-GRAPH-GET-NODES-BATCH-031",
    "DATA-AGE-GRAPH-UPSERT-NODES-BATCH-046",
    "DATA-PG-KV-GET-BY-ID-075",
    "DATA-PG-KV-GET-BY-IDS-076",
    "DATA-PG-KV-UPSERT-079",
    "DATA-PG-TASKS-CLAIM-NEXT-140",
]
src_blob = ""
for p in (ROOT / "edgequake/crates").rglob("*.rs"):
    if "/tests/" in str(p) or p.name.startswith("dataop"):
        continue
    try:
        src_blob += p.read_text(errors="replace")
    except Exception:
        pass
for h in hot:
    if f"@dataop      {h}" not in src_blob:
        errors.append(f"Hot-path missing inline @dataop block: {h}")


# Every inventory Ref ID must appear in the generated ops matrix test names
matrix = ROOT / "edgequake/crates/edgequake-storage/tests/data_layer_ops_matrix.rs"
if not matrix.exists():
    errors.append("missing data_layer_ops_matrix.rs")
else:
    mx = matrix.read_text()
    missing_tests = [r for r in sorted(refs) if r.lower().replace("-", "_") not in mx]
    if missing_tests:
        errors.append(f"Refs missing from data_layer_ops_matrix tests: {missing_tests[:5]}… ({len(missing_tests)})")

if errors:
    print("SPEC-088 dataop lint FAILED:")
    for e in errors:
        print(" -", e)
    sys.exit(1)
print(
    f"SPEC-088 dataop lint OK: {len(refs)} refs inventory=code=catalog; "
    f"docs cover {len(doc_refs & refs)}/{len(refs)}; hot-path inline={len(hot)}"
)
