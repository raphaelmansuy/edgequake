# SPEC-012 — Storage performance, iteration 01 audit

> **Status**: implemented (Fixes B', C+, E partial, F, H) — see `ITERATION_01_PROOF.md`.
> **Cross-ref**: [`specs/11-performance-issue/ITERATION_02_AUDIT.md`](../11-performance-issue/ITERATION_02_AUDIT.md), [`specs/11-performance-issue/ITERATION_02_PLAN.md`](../11-performance-issue/ITERATION_02_PLAN.md).
> **Branch**: `fix/spec-011-storage-performance`.
> **Evidence**: [`data/queryedgeQuake.csv`](data/queryedgeQuake.csv) — 31 286 PostgreSQL log rows.

## 1. Why this iteration exists (first principles)

SPEC-011 iter 02 was planned from code reading. The user demanded the optimisation be **grounded in real production data**. That CSV is a PostgreSQL log export from a running EdgeQuake deployment. Re-deriving priorities from it changed the ranking and surfaced **two hotspots the iter 02 plan missed**:

1. `SELECT value FROM kv WHERE key = ANY($1)` — 33.3 s total (32.8% of DB time).
2. `SELECT key FROM kv` — 9.4 s total (9.2% of DB time).

It also produced an *operational* finding: the deployment is still running the iter-02 fallback path of `count()`, which means the SPEC-011 stats table was never bootstrapped on that database. That is a deployment / migration class of bug, fixable by **self-healing the fallback**.

> "Code is law" — but production logs are law squared. They are the only oracle for what is *actually* slow.

## 2. Method

```bash
# Parse CSV, normalize query texts (strip whitespace, $-numbers, quoted literals),
# group by canonical form, sum total_exec_ms and count calls, rank by total_exec_ms.
python3 scripts/analyze_eq.py data/queryedgeQuake.csv > eq_out.txt
```

- Source: `timestamp,source,message,detail,duration_num,query,dbname` from PostgreSQL `log_min_duration_statement` capture.
- 31 286 rows with non-null `duration_num`. Total DB time observed: **101.4 s**.
- 160 distinct normalised queries; top 8 account for **>96%** of total DB time.
- Workload mix: SELECT **97.62%** of DB time; INSERT/UPDATE/DELETE combined **<0.5%**. **This is a read-bound, polling-driven workload.**

## 3. Empirical hotspots (raw, ranked by total exec time)

| Rank | Total ms | Calls | Mean ms | Max ms | Canonical query | Root cause | Fix in this iteration |
|-----:|---------:|------:|--------:|-------:|----------------|------------|-----------------------|
| 1 | **38 152** (37.6%) | 5 780 | 6.6 | — | `SELECT COUNT(*)::bigint FROM eq_eq_default_graph."_ag_label_vertex"` | Exact vertex count, polled by graph endpoints | **Fix B** (planner estimate via `pg_class.reltuples`) — SPEC-011 iter 02 |
| 2 | **33 275** (32.8%) | 1 164 | 28.6 | 378 | `SELECT value FROM public.eq_eq_default_kv WHERE key = ANY($1)` | `get_by_ids()` hot path; large JSONB values | **Fix E** (caller migration: shrink ID lists by upstream `keys_with_suffix`) |
| 3 | **12 078** (11.9%) | 5 806 | 2.1 | — | `SELECT COUNT(*) as count FROM public.eq_eq_default_kv` | Fallback path of SPEC-011 `count()` — stats row missing | **Fix H** (self-heal: bootstrap stats on miss) |
| 4 | **9 357** (9.2%) | 1 166 | 8.0 | 484 | `SELECT key FROM public.eq_eq_default_kv` | `kv.keys()` full-table dump used by polled handlers + filtered with `.ends_with("-metadata")` / `.starts_with("auth:user:")` | **Fix C+** / **Fix F** (replace with `keys_with_suffix` / `keys_with_prefix`) |
| 5 | 1 348 (1.3%) | 5 775 | 0.23 | — | `SELECT COUNT(*) FROM public.eq_eq_default_vectors` | Exact vector count, polled | **Fix A** (maintained counter) — SPEC-011 iter 02 |
| 6 | 1 172 (1.2%) | 5 880 | 0.20 | — | `_sqlx_migrations` poll | Per-request health check | Fix D — deferred (cost / call too low to justify cache invalidation risk) |
| 7 | 909 | 5 | 182 | — | `MATCH (n:Node) RETURN n` (Cypher) | Full node enumeration | Out of scope (query layer) |
| 8 | 890 | 3 | 297 | — | `MATCH ()-[r:EDGE]->() RETURN r` (Cypher) | Full edge enumeration | Out of scope |

**Observation**: rows 1, 3, 5, 6 are each called ≈5 800 times — that is the **frontend polling cycle** (`staleTime: 30000` in the WebUI's React Query config). Polling × exact O(N) reads is the dominant load. The fix philosophy is therefore: **make every polled query O(1) or O(log N) — without exception**.

## 4. Mapping each empirical hotspot to a fix

### Hotspot 1 — `_ag_label_vertex COUNT(*)` (37.6% of DB time)

**Code origin**: `graph_query/popular.rs:33`, `graph/graph_stream.rs:82-83`, `graph_query/traversal.rs:260-261`.

**Fix B** introduced `GraphStorage::node_count_fast()` / `edge_count_fast()` with a Postgres override that reads `pg_class.reltuples` (planner estimate; constant time). The trait default delegates to the exact methods so non-Postgres backends are unaffected. All three polled callers were migrated.

**Trade-off**: `reltuples` lags between `ANALYZE` cycles. For *display in a UI*, an estimate within ±5% is indistinguishable to humans. The exact `node_count()` / `edge_count()` methods remain available for callers that need exact counts (e.g. tests, billing) — no semantic regression.

**Index touched**: none. Cost: zero migration writes.

### Hotspot 2 — `get_by_ids(metadata_keys)` (32.8% of DB time)

**Code origin** (all callers of `kv.get_by_ids`):

- `handlers/costs.rs:154,380` — cost dashboard, polled.
- `handlers/tasks.rs:235` — task cancel + status update.
- `handlers/query/document_filter_resolver.rs:56` — per-query metadata join.

**Fix E (this iteration)** — the dominant amplifier is *input size*. Each `get_by_ids` call is preceded by either `kv.keys() + filter` or `kv.keys_like("%-metadata")`, which together produced **every key in the kv table** before fetching its value. After Fix C+ (below) shrinks the upstream key set to *only* metadata keys via an indexed scan, the `get_by_ids` array shrinks proportionally and the per-call cost falls.

**Deferred (next iteration)**: schema-level — if the mean still exceeds ~5 ms after this iteration, the `value` column should be split (metadata vs heavy content) to avoid TOAST detoasting on dashboard reads.

**No adapter change in this iteration** — the win comes from caller-side reduction of `metadata_keys.len()`.

### Hotspot 3 — `SELECT COUNT(*) as count FROM kv` (11.9% of DB time)

**Code origin**: `adapters/postgres/kv.rs::count` — *fallback path* of the SPEC-011 maintained-counter optimisation.

**Production observation**: this query should be **dead code** on a healthy SPEC-011 deployment. The fact that it ran 5 806 times proves the `eq_eq_default_kv_stats` row was missing on that database (likely a legacy table predating iter 02 init).

**Fix H** — make the fallback self-healing:

```rust
// SPEC-012 Fix H: bootstrap stats on first miss so legacy deployments auto-upgrade.
tracing::warn!(stats_table = %self.stats_table_name,
               "KV stats row missing — running self-heal");
let _ = self.ensure_row_count_stats(&pool).await;
```

On the second `count()` call the primary path returns. The same pattern was applied to `vector::count`.

**Trade-off**: one extra `COUNT(*) + INSERT` per affected deployment, only ever once. Worth it to guarantee the SPEC-011 invariant holds in the field.

### Hotspot 4 — `SELECT key FROM kv` (9.2% of DB time)

**Code origin** (production-hot callers of `kv.keys()` followed by a filter):

| File | Filter | Iteration-01 fix |
|------|--------|------------------|
| `handlers/tasks.rs:228` | `.ends_with("-metadata")` | `keys_with_suffix("-metadata")` |
| `handlers/auth/user_management.rs:204,370` | `.starts_with(USER_KEY_PREFIX)` | `keys_with_prefix(USER_KEY_PREFIX)` |
| `handlers/costs.rs:144,374` | `keys_like("%-metadata")` (already a wildcard scan) | `keys_with_suffix("-metadata")` |
| `handlers/workspaces/stats.rs:160` | `keys_like("%-metadata")` | `keys_with_suffix("-metadata")` — done in SPEC-011 iter 02 |
| `handlers/documents/delete/bulk.rs:44` | `keys_like("%-metadata")` | `keys_with_suffix("-metadata")` |

**Deliberately *not* migrated** (one-shot / non-polled / requires full-key universe):

| File | Reason |
|------|--------|
| `handlers/workspaces/workspace_crud.rs:374` (delete workspace) | Needs full key set to find chunk keys per document; rare admin op. |
| `handlers/documents/delete/single.rs:77` | Single-document delete; not polled. |
| `handlers/injection.rs:383,500` | Injection one-shots; not polled. |
| `processor/pdf_processing.rs:280` | Boot-time scan; not polled. |
| `core/orchestrator/deletion.rs:44,194` | Deletion path; not polled. |

The migration is conservative — every change preserves the same logical key set, only the *path* changes from "full table scan in Rust" to "indexed scan in Postgres".

**Index supporting `keys_with_suffix`**:

```sql
CREATE INDEX IF NOT EXISTS eq_{prefix}_kv_reverse_key_idx
    ON {kv_table} (reverse(key) text_pattern_ops);
```

Expression index on `reverse(key)`; suffix matches become prefix matches on the reversed string and use the index. `text_pattern_ops` ensures ASCII-collation index usability for `LIKE`.

## 5. Risk analysis (Big-O × call-frequency)

| Query | Before | After | Calls/30 s cycle | Time saved per cycle (proj.) |
|-------|--------|-------|------------------|------------------------------|
| `_ag_label_vertex COUNT(*)` | O(N_v) seq scan | O(1) catalog read | ~3 | ~20 ms |
| `_ag_label_edge COUNT(*)` | O(N_e) seq scan | O(1) catalog read | ~3 | ~15 ms |
| `kv COUNT(*)` | O(N_k) seq scan | O(1) PK lookup | ~3 | ~6 ms |
| `vectors COUNT(*)` | O(N_v) seq scan | O(1) PK lookup | ~3 | ~1 ms |
| `kv.keys()` full dump | O(N_k) seq scan + net | O(log N_k + K) index range | ~1 | ~8 ms (and ↓ proj. `get_by_ids` arg size) |

Across a 30 s polling window the projected reduction is **~50 ms of Postgres time per cycle per worker**, plus a payload reduction on `get_by_ids` that compounds. Over a 24 h window the projected total reduction is **~12 GB of unnecessary key bytes transferred** (rough: 1 166 polls × ~10 KB average key payload).

## 6. Non-regression contract

| Surface | Guarantee | How verified |
|---------|-----------|--------------|
| `KVStorage::keys_with_suffix` | Returns identical set to `keys() + filter ends_with(suffix)` for ASCII suffixes | Provider compatibility test (memory ↔ postgres) |
| `GraphStorage::node_count_fast` | May lag by ANALYZE cycle; same monotonic behaviour for empty graphs | Memory backend test: `node_count_fast == node_count` since default delegates |
| `KVStorage::count` self-heal | Never returns wrong value; only spends one extra query on legacy deployments | Manual: drop stats table, call `count()`, observe warning + correct value |
| Workspace deletion | Behavioural identity preserved | Unchanged code path |
| Trait additions | Default impl delegates to existing methods | Backwards-compatible by construction |

All 268 workspace lib tests pass; `cargo clippy -p edgequake-storage -p edgequake-api --all-targets -- -D warnings` is clean.

## 7. What is *not* fixed in iteration 01

- **Cypher full-scan `MATCH (n) RETURN n`** (rows 7-8): handled by the query layer, separate concern.
- **`_sqlx_migrations` health-check polling** (row 6): only 1.2% of DB time, cache invalidation complexity not justified.
- **`get_by_ids` schema split**: needed only if mean stays > 5 ms after this iteration; revisit with fresh CSV.
- **The remaining one-shot `keys()` callers**: deletion / injection / boot paths; not polled.

## 8. Files modified in iteration 01

```text
edgequake/crates/edgequake-storage/src/traits/graph.rs       (+node_count_fast, +edge_count_fast)
edgequake/crates/edgequake-storage/src/traits/kv.rs          (+keys_with_suffix)
edgequake/crates/edgequake-storage/src/adapters/postgres/
    graph/mod.rs                                              (+reltuples_estimate + overrides)
    kv.rs                                                     (+reverse-key index, +self-heal, +keys_with_suffix)
    vector.rs                                                 (+stats table + triggers, +self-heal)
edgequake/crates/edgequake-api/src/handlers/
    graph/graph_stream.rs                                     (use *_count_fast)
    graph/graph_query/popular.rs                              (use *_count_fast)
    graph/graph_query/traversal.rs                            (use *_count_fast)
    workspaces/stats.rs                                       (use keys_with_suffix)
    tasks.rs                                                  (use keys_with_suffix)
    auth/user_management.rs                                   (use keys_with_prefix)
    costs.rs                                                  (use keys_with_suffix, both call sites)
    documents/delete/bulk.rs                                  (use keys_with_suffix)
```

## 9. Next iteration (proposed)

1. Capture a fresh CSV after deploying iter 01. Confirm hotspots 1, 3, 4, 5 collapse.
2. Re-rank. If `get_by_ids` mean is still > 5 ms, split `value` column.
3. Investigate Cypher full-scan callers (`MATCH (n:Node) RETURN n`).
4. Add `pg_stat_statements` to the deployment so the next audit can run without log scraping.
