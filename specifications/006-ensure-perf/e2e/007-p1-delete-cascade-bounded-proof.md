# E2E Proof 007 — P1 Document Delete Cascade Bounded

**Spec:** SPEC-006 P1 · `V-006-002`  
**Requirement:** NFR-006-002, TR-006-002, UC-006-002  
**Status:** ✅ Verified 2026-06-06

---

## First Principle

> Delete must touch **only graph objects whose `source_ids` reference the document** — never `get_all_nodes()` × workspace size.

`PeakRAM(delete) ≈ O(document_entities + document_edges + orphan_degree)` — not `O(workspace_nodes)`.

---

## Code Is Law

| Layer | Artifact | Proof |
|-------|----------|-------|
| SRP service | `edgequake-api/src/services/document_graph_cascade.rs` | `cascade_remove_document_sources` |
| Handler | `handlers/documents/delete/single.rs` | Calls service, no `get_all_*` |
| DRY | `handlers/documents/storage_helpers.rs` | `cleanup_document_graph_data` delegates |
| Bulk | `handlers/documents/delete/bulk.rs` | Per-doc cascade, no post-hoc full scan |
| Impact | `handlers/documents/delete/impact.rs` | `analyze_deletion_impact_stats` |

---

## Automated Proof

```bash
cargo test -p edgequake-api resource_safety_delete_cascade_bounded_scope -- --nocapture
```

**Scenario:** 500 unrelated workspace nodes + 3 document-scoped entities + 1 edge.

**Assertions:**
- `entities_removed == 2` (doc-only entities)
- `entities_updated == 1` (shared entity keeps other-doc source)
- `PROOF_ENTITY_000000` still exists (noise node untouched)
- Document edge removed

---

## Allowlist Gate

`documents/delete/*` and `storage_helpers.rs` removed from `support/get_all_allowlist.txt`.
