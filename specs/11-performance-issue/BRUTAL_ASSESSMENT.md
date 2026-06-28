# Brutal Honest Assessment — SPEC-011 Storage Performance

> **Verdict**: Phase 1 fixed a real production fire. Phase 1 did **not** make EdgeQuake storage fast end-to-end.
> We removed the dumbest mistake (COUNT for health). Most hot paths still scale linearly with table size.

---

## What we actually fixed (and can stand behind)

| Fix | Honest impact | Guaranteed? |
| --- | ------------- | ----------- |
| `/health` uses `ping()` not `count()` | Eliminates 13s probes on large KV tables | **Yes** — O(1) query, provable |
| `is_empty()` uses EXISTS | Boolean check without full count | **Yes** |
| Shared connection pool | Cuts max connections from 3×N to N | **Yes** — wiring change |
| Batch KV upsert (unnest) | Fewer round-trips during ingestion | **Yes** — measurable RTT reduction |
| `keys_like` on list/stats/track | Smaller result sets over the wire | **Partial** — see below |

---

## What we did NOT fix (be honest)

### 1. `keys_like('%-metadata')` is still often O(N) in PostgreSQL

A **leading wildcard** (`%-suffix`) cannot use the primary-key B-tree index. PostgreSQL still scans the heap (or index) and evaluates LIKE per row. We reduced **application-side** work (smaller Vec, less filtering in Rust) but **not** guaranteed DB CPU/IO reduction for suffix patterns.

**Truth**: For 100k rows, `keys_like('%-metadata')` may still take seconds on cold cache — just less than transferring 100k keys to Rust.

**Real fix for suffix queries**: expression index on `reverse(key)`, `pg_trgm`, or a sidecar `documents` table (normalized schema).

### 2. ~15 call sites still use `keys()` full scan

After phase 1, these remained (partial list):

- `costs.rs`, `bulk delete`, `detail.rs`, `impact.rs`, `lineage`, `storage_helpers`, `main.rs`, `pipeline_checkpoint`, `orchestrator/deletion`, `pdf_processing`, `tasks.rs`, `injection.rs`, `workspace_crud`, recovery handlers

Any of these can reproduce the production pain under load.

### 3. Graph operations are still expensive

- `node_count()` / `edge_count()` remain O(N) — only removed from health
- `get_all_nodes()` in lineage is O(N) fetch over network
- Workspace graph counts still use Cypher, not native indexed SQL

### 4. We have no production SLO enforcement

Tests pass on memory backend where `count()` is O(1) HashMap len. That **hides** the PostgreSQL regression class entirely unless `POSTGRES_PASSWORD` is set. CI likely never runs postgres perf tests.

### 5. Default trait `ping()` still calls `count()` on unoptimized adapters

Memory adapters inherited default until overridden. Misleading for anyone implementing a new backend.

### 6. Vector upsert still row-by-row

We batch-fixed KV only. Vector ingestion still loops INSERT per embedding.

---

## First-principle: what "guaranteed performance" actually means

Performance is not a vibe. It requires **contracts**:

```
∀ operation O in hot_path:
  complexity(O) ∈ allowed_class
  latency(O, n=reference_load) < SLO(O)
  CI test proves both
```

Without all three, you have **hope**, not a guarantee.

| Layer | What guarantees latency |
| ----- | ------------------------ |
| Algorithm | O(1) ping, prefix scan not full scan |
| Database | B-tree prefix (`key LIKE 'doc%'` not `'%meta'`) |
| Architecture | Normalized tables for counts, not KV enumeration |
| Operations | Pool size, statement timeouts, index migrations |
| Verification | E2E tests with seeded load + ms thresholds |

We only had layer 1 partially, zero on layers 2–5 before this follow-up.

---

## Risk if we stop here

| Scenario | Outcome |
| -------- | ------- |
| Health fixed, dashboard polling list + stats | Still heavy under 100k chunks |
| Operator runs "Clear all" / cost summary | Full `keys()` scan |
| Document detail page | Full `keys()` to count chunks |
| Next incident | "Why is `SELECT key FROM eq_eq_default_kv` slow?" |

---

## Phase 2 direction (this follow-up)

1. Add **`keys_with_prefix`** — B-tree-friendly (`LIKE 'prefix%'`)
2. Migrate all document-scoped and metadata suffix call sites
3. Override memory adapter `ping`, `keys_like`, `keys_with_prefix` without full scan
4. Publish **`PERFORMANCE_GUARANTEE.md`** — explicit SLO table + what is NOT guaranteed
5. Add **`e2e_storage_performance_spec011`** — seeded load + latency assertions

See [PERFORMANCE_GUARANTEE.md](./PERFORMANCE_GUARANTEE.md) and [IMPROVEMENT_PLAN.md](./IMPROVEMENT_PLAN.md) phase 2.
