# `GH-331` — Pool exhaustion on document reprocess (source_ids GIN locality)

> **Priority**: P0  
> **Audit status**: FIXED  
> **Sprint**: 0  
> **Laws**: LAW-9, LAW-12, LAW-8  
> **GitHub**: https://github.com/raphaelmansuy/edgequake/issues/331  
> **Verified against**: v0.21.0 / `19477c2d`

---

## 1. WHY

At scale (~130k vertices, ~9k docs), reprocess / list reconcile runs a JSONB `@>` containment query that holds a DB connection for minutes. Concurrent tasks exhaust the pool (`pool timed out while waiting for an open connection`), taking down health checks, task claims, and UI polling.

Reporter impact is correct. The suggested fix (GIN on `_ag_label_vertex`) is **not** the correct law under AGE inheritance.

---

## 2. Audit (code is law)

| Field | Value |
|-------|-------|
| Primary locus | `edgequake/crates/edgequake-storage/src/adapters/postgres/graph/analytics_ops.rs:193-216` |
| Child GIN | M038 `idx_node_source_ids_gin` on `"Node"` — **exists** |
| Parent GIN | Reporter `idx_age_vertex_source_ids` — **absent**; M070 drops parent indexes |
| Discovery path | `scan_ops.rs` retargeted to `"Node"` (SPEC-071) — **fixed** |
| Count path | JOINs `"Node"` (SPEC-084) — **fixed** |
| Verdict | **FIXED** — child GIN locality; parent GIN rejected |

```sql
-- FIXED (analytics_ops.rs) — SPEC-084 / LAW-9
JOIN {graph}."Node" v
  ON ((ag_catalog.agtype_to_json(v.properties))::jsonb -> 'source_ids')
     @> to_jsonb(pr.chunk_id)
```

```sql
-- M038 (correct locality)
CREATE INDEX IF NOT EXISTS idx_node_source_ids_gin ON %I."Node"
USING gin ((ag_catalog.agtype_to_json(properties)::jsonb -> 'source_ids') jsonb_ops);
```

Callers of `node_counts_by_source_prefixes` include document list entity-count reconcile (`document_read_model.rs`) — same pool risk as reprocess when many docs reconcile concurrently.

---

## 3. Root cause (first principles)

Apache AGE stores labeled vertices on **child tables** (`"Node"`). Parent `_ag_label_vertex` is an inheritance root with ~0 application rows after M070. GIN for `source_ids` was correctly placed on `"Node"` (M038). The count query still joins the parent → planner cannot use child GIN → Seq Scan / Nested Loop over probes × vertices → multi-minute connection hold → pool exhaustion.

Invariant violated: **LAW-9 Index locality**.

---

## 4. Multi-lens analysis

### Product Owner

- Acceptance: Retry Failed / reprocess at 5k+ docs must leave `/health` and task claim healthy.
- Do not ship a migration that recreates parent indexes (false progress; conflicts with M070).
- Operator workaround until fixed: kill long queries; run reconcile with lower concurrency — not acceptable as product.

### Full Stack

| Layer | Finding |
|-------|---------|
| API | Reprocess admits tasks; list reconcile also calls count API |
| Storage | `analytics_ops` count = parent; `scan_ops` discovery = child |
| Pool | Default `DATABASE_POOL_SIZE=32`, acquire timeout 5s — cannot absorb 2+ min holds |
| FE | “Retry Failed” amplifies concurrency into the bad query |

### AI Engineer

- Not an LLM bug. Entity extraction may *trigger* reprocess, but the stall is pure SQL.

### O(n) / Systems

- Probes: `prefixes × SOURCE_CHUNK_PROBE_LIMIT` (256) `@>` checks.
- Without index: ≈ O(probes × |V|) per document reconcile.
- With child GIN: ≈ O(probes × log |V|) / bitmap.
- Concurrent tasks × long holds → pool deadlock-of-the-living (starve short queries).

### Postgres Expert

- AGE inheritance: indexes on parent do not cover child heap the way reporters expect; M070 documents parent indexes as dead weight.
- Existing M038 uses `jsonb_ops` on the **`source_ids` array expression** — correct for `@> to_jsonb(chunk_id)` ([PostgreSQL GIN](https://www.postgresql.org/docs/current/gin.html)).
- Reporter’s full-properties `jsonb_path_ops` on parent is both wrong table and wrong shape vs the expression already indexed.
- Fix: change JOIN target to `"Node"`; EXPLAIN must show Bitmap Index Scan on `idx_node_source_ids_gin`.
- Optional later: `jsonb_path_ops` on the same expression if write amplification matters — not required to close #331.

---

## 5. ASCII causal diagram

```
  M038 GIN on "Node"
        |
        v
  count SQL JOINs _ag_label_vertex  -->  Seq Scan / no GIN
        |                                      |
        v                                      v
  connection held 2+ min  ---------->  pool acquire timeout
        |
        v
  health / claim / UI 500-503
```

---

## 6. Solution (SOLID + DRY)

| Principle | Application |
|-----------|-------------|
| S | `analytics_ops` owns count SQL; must match `scan_ops` table choice |
| O | Shared helper for “source_ids `@>` probe join FROM clause” |
| L | Count and discovery honor same child-table contract |
| I | Trait `node_counts_by_source_prefixes` unchanged externally |
| D | Depend on M038 index name SSOT; do not invent parent index |
| DRY | Extract table+expression from SPEC-071 `scan_ops` / `source_lineage_sql` |

### Implementation steps

1. In [`analytics_ops.rs`](../../../edgequake/crates/edgequake-storage/src/adapters/postgres/graph/analytics_ops.rs), change JOIN to `{graph}."Node"` (mirror `scan_ops.rs` modern_sql).
2. Audit other `_ag_label_vertex` `@>` / `source_ids` paths in the same module; retarget or justify.
3. Update EXPLAIN gates in mix-scale perf tests that still plan against parent.
4. **Do not** add `idx_age_vertex_source_ids` on `_ag_label_vertex`.
5. No new migration required if M038 already applied; boot reconcile already ensures child GIN.

---

## 7. Edge cases

| ID | Case | Mitigation |
|----|------|------------|
| EC-1 | Graph created before M038 (missing GIN) | Boot reconcile `m038.rs` + graph_lifecycle create path |
| EC-2 | Legacy vertices only on unexpected labels | Keep SPEC-071 legacy fallback pattern if present for discovery; document for count |
| EC-3 | Empty `source_ids` | Count 0; no scan blowup |
| EC-4 | Probe limit 256 undercounts huge docs | Pre-existing probe cap; out of scope except document |
| EC-5 | Concurrent reprocess 16+ tasks | Child GIN + existing pool; add e2e that acquire_timeout does not fire |
| EC-6 | Operator applied reporter’s parent GIN manually | Harmless but useless; do not depend on it; M070 may drop siblings |

---

## 8. E2E / contract tests

| Test | Assertion |
|------|-----------|
| `issue331_node_counts_uses_child_gin_explain` | EXPLAIN of count SQL contains `idx_node_source_ids_gin` and `"Node"`; must not Seq Scan parent heap for hits |
| `issue331_concurrent_reprocess_pool_stable` | N concurrent prefix-count calls complete under acquire_timeout; `/health` stays healthy |
| `issue331_parity_count_vs_discovery` | For seeded prefixes, count == `find_nodes_by_source_prefixes`.len() |

---

## 9. Cross-refs

- SPEC-071 lineage discovery (child table retarget)  
- M038 / M070 migrations  
- SPEC-083 LAW-8 (tests prove production defaults)  
- Related: #317 (same pool class of failure under N× cascades)
