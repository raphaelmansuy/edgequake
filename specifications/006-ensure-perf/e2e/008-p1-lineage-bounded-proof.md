# E2E Proof 008 — P1 Lineage Bounded Queries

**Spec:** SPEC-006 P1 · `V-006-004`  
**Requirement:** NFR-006-002, TR-006-003  
**Status:** ✅ Verified 2026-06-06

---

## First Principle

Lineage answers "what did **this document/chunk/entity** extract?" — a **prefix-scoped** query, not a workspace graph export.

---

## Code Is Law

| Endpoint | Before | After |
|----------|--------|-------|
| `GET /lineage/documents/{id}` | `get_all_nodes` + `get_all_edges` | `find_document_nodes/edges` |
| `GET /chunks/{chunk_id}` | full graph scan | chunk prefix scope |
| `GET /entities/{id}/provenance` (related) | `get_all_edges` | `get_node_edges(id)` |

---

## Automated Proof

```bash
cargo test -p edgequake-api resource_safety_delete_cascade_bounded_scope --quiet
cargo test -p edgequake-storage graph_scan_ops --quiet
```

Lineage handlers compile against `DocumentSourceScope` + `GraphScanOps` — static proof that `get_all_*` is absent (enforced by `scripts/spec006_no_get_all_api.sh`).

---

## Allowlist Gate

`lineage/queries.rs`, `lineage/chunk_detail.rs`, `lineage/entity_provenance.rs` removed from allowlist.
