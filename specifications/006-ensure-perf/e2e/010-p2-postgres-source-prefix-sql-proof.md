# E2E Proof 010 — P2 Postgres Source-Prefix SQL Push-Down

**Spec:** SPEC-006 P2  
**Requirement:** TR-006-005, NFR-006-003  
**Status:** ✅ Verified 2026-06-06 (static + memory; Postgres SQL reviewed)

---

## First Principle

> `find_by_source_prefixes` must filter **in SQL**, not load 100k tenant rows into Rust.

---

## Code Is Law

`edgequake-storage/src/adapters/postgres/graph/scan_ops.rs`:

- `build_source_prefix_clause` — `source_id` LIKE + `source_ids` jsonb array scan
- `pg_find_nodes_by_source_prefixes` — tenant WHERE + source clause (no 100k page)
- `pg_find_edges_by_source_prefixes` — same for edges

---

## Proof Method

1. **Static:** Grep confirms no `usize::MAX.min(100_000)` in prefix find paths
2. **Memory regression:** `graph_scan_ops_find_by_source_prefix`
3. **Integration:** `resource_safety_delete_cascade_bounded_scope` (cascade uses prefix find)

```bash
rg '100_000' edgequake/crates/edgequake-storage/src/adapters/postgres/graph/scan_ops.rs
# Expected: no matches in find_nodes/edges_by_source_prefixes bodies
cargo test -p edgequake-storage graph_scan_ops --quiet
```

Postgres runtime proof requires `DATABASE_URL` integration test (optional CI job).
