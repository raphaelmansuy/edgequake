# E2E Proof 009 — P2 Relationship Lookup Bounded

**Spec:** SPEC-006 P2 · `V-006-001`  
**Requirement:** TR-006-004  
**Status:** ✅ Verified 2026-06-06

---

## First Principle

Relationship get/update/delete must be **O(1) edge lookup** by id — never `get_all_nodes()` × `get_node_edges()`.

---

## Code Is Law

| Component | Change |
|-----------|--------|
| `GraphScanOps::find_edge_by_relationship_id` | SQL `LIMIT 1` (Postgres) / single-pass (Memory) |
| `relationships/helpers.rs` | `find_relationship_edge` (DRY) |
| `relationships/get\|delete\|update.rs` | Delegate to helper + `TenantContext` |

Relationship id matches `source_target` composite **or** edge property `id`.

---

## Automated Proof

```bash
cargo test -p edgequake-storage graph_scan_ops_find_edge_by_relationship_id --quiet
cargo test -p edgequake-api resource_safety_relationship_lookup_bounded --quiet
./scripts/spec006_no_get_all_api.sh
```

Allowlist: `relationships/*` removed — **zero** `get_all_*` patterns in allowlist.
