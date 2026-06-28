# Improvement Plan — SPEC-011 Storage Performance

Phased implementation with edge cases, mitigations, and non-regression gates.

---

## Phase 1 — Stop the Bleeding (O(1) probes)

### Fix 1A: Add `ping()` to storage traits

**Trait changes** (`KVStorage`, `VectorStorage`, `GraphStorage`):

```rust
async fn ping(&self) -> Result<()> {
    let _ = self.count().await?; // default: backward compat for memory/tests
    Ok(())
}
```

**Postgres overrides**:
- KV/Vector: `SELECT 1 FROM {table} LIMIT 1`
- Graph: `SELECT 1 FROM {graph}."Node" LIMIT 1` (falls back to vertex table if empty)

| Edge case | Mitigation |
| --------- | ---------- |
| Empty table | `LIMIT 1` on empty table returns 0 rows — use `SELECT 1` without FROM or EXISTS on pg_catalog |
| Graph table not created (AGE missing) | ping returns Err → health shows degraded (same as today) |
| Memory adapter | Default impl calls count() — O(1) in memory |

**Non-regression**: Existing `count()` tests unchanged. New `ping()` tests added.

### Fix 1B: Health check uses `ping()`

```rust
// health.rs — before
kv_storage: state.kv_storage.count().await.is_ok(),

// after
kv_storage: state.kv_storage.ping().await.is_ok(),
```

| Edge case | Mitigation |
| --------- | ---------- |
| ping succeeds but count would fail | ping uses same pool/connection — if table missing, both fail |
| Semantic change | Health only checks connectivity, not cardinality — **correct behavior** |

### Fix 1C: `is_empty()` uses EXISTS

```sql
SELECT NOT EXISTS (SELECT 1 FROM {table} LIMIT 1)
```

| Edge case | Mitigation |
| --------- | ---------- |
| Empty table | Returns true — same as count()==0 |
| Concurrent insert during check | EXISTS may return false (non-empty) — acceptable race |

---

## Phase 2 — Filtered Key Scans

### Fix 2A: Add `keys_like(pattern: &str)`

**Postgres**:
```sql
SELECT key FROM {table} WHERE key LIKE $1
```

**Memory default**: `keys().await` then filter — acceptable for tests.

**SQL LIKE escaping**: Document keys use UUIDs and `-` — no `%` or `_` in keys. Patterns are server-controlled (`"%-metadata"`, `"%-chunk-%"`), not user input.

| Edge case | Mitigation |
| --------- | ---------- |
| User-controlled LIKE pattern | Not exposed via API — patterns hardcoded in handlers |
| `_` wildcard in doc IDs | UUIDs don't contain `_`; chunk keys use `-chunk-` delimiter |
| `%` in injection keys | `injection::` prefix excluded by pattern (doesn't match `%-metadata`) |
| Performance on `%`-prefix | No leading wildcard → can use index if added later; current PK scan still filters early |

### Fix 2B: Update hot-path handlers

| Handler | Before | After |
| ------- | ------ | ----- |
| `list.rs` | `keys()` | `keys_like("%-metadata")` + `keys_like("%-chunk-%")` |
| `track_status.rs` | `keys()` | same |
| `workspaces/stats.rs` | `keys()` filter | `keys_like("%-metadata")` |

| Edge case | Mitigation |
| --------- | ---------- |
| Missing chunk keys in list | Still fetched via separate LIKE query |
| Documents without metadata key | Orphan chunks ignored (same as before) |

---

## Phase 3 — Batch Upsert

### Fix 3A: unnest batch INSERT

```sql
INSERT INTO {table} (key, value, updated_at)
SELECT k, v, NOW()
FROM unnest($1::text[], $2::jsonb[]) AS t(k, v)
ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = NOW()
```

| Edge case | Mitigation |
| --------- | ---------- |
| Empty batch | Early return (unchanged) |
| Single item | Still uses batch path — same semantics |
| Partial failure | Transaction wraps batch — all or nothing |
| Very large batch (>10k) | Chunk into 1000-row sub-batches to avoid param limits |

---

## Phase 4 — Shared Connection Pool

### Fix 4A: `PostgresPool::from_existing(pool, config)`

Wire in `AppState::new_postgres()`:

```rust
let storage_pool = PostgresPool::from_existing(pool.clone(), pg_config.clone());
let kv_storage = Arc::new(PostgresKVStorage::with_pool(storage_pool.clone(), pg_config.clone()));
```

Add `with_pool()` constructors to each postgres adapter.

| Edge case | Mitigation |
| --------- | ---------- |
| Double initialize extensions | `from_existing` skips extension setup (already done on main pool) |
| Pool closed on shutdown | All adapters share lifecycle |
| Tests using `PostgresKVStorage::new(config)` | Still work — own pool for isolation |

---

## Phase 5 — Future (documented, not in this PR)

| Item | Approach |
| ---- | -------- |
| Exact count caching | `pg_stat_user_tables.n_live_tup` with TTL for dashboard only |
| `keys()` deprecation | Migrate remaining callers in `main.rs`, `orchestrator`, `injection`, `tasks`, `pdf_processing`, `workspace_crud`, recovery handlers |
| Graph workspace counts | Native SQL on AGE property indexes (migration 014) |
| PDF list without blob | Separate `list_pdfs_metadata` query |
| Maintained counter table | Trigger-based `eq_storage_stats` if exact count needed on hot path |
| Suffix index for `%-metadata` | `reverse(key)` expression index or normalized `documents` table |

---

## Phase 2 — Completed in follow-up (keys_with_prefix + E2E SLO)

| Fix | Status |
| --- | ------ |
| `keys_with_prefix()` B-tree friendly API | ✅ |
| Migrate detail, impact, bulk, lineage, costs, storage_helpers, checkpoint | ✅ |
| Memory adapter ping/keys overrides | ✅ |
| `e2e_storage_performance_spec011` latency tests | ✅ |
| [BRUTAL_ASSESSMENT.md](./BRUTAL_ASSESSMENT.md) + [PERFORMANCE_GUARANTEE.md](./PERFORMANCE_GUARANTEE.md) | ✅ |

---

## Non-Regression Test Matrix

| Test suite | Command | Must pass |
| ---------- | ------- | --------- |
| Storage unit | `cargo test -p edgequake-storage --lib` | ✅ |
| Storage integration | `cargo test -p edgequake-storage --test postgres_integration` | ✅ (if PG available) |
| E2E backends | `cargo test -p edgequake-storage --test e2e_storage_backends` | ✅ |
| Dashboard stats | `cargo test -p edgequake-api --test e2e_dashboard_stats_issue81` | ✅ |
| Health handler | `cargo test -p edgequake-api health` | ✅ |
| Performance (new) | `cargo test -p edgequake-storage --test performance_storage` | ✅ |
| Clippy | `cargo clippy -p edgequake-storage -p edgequake-api -- -D warnings` | ✅ |

### Performance acceptance criteria

| Metric | Before | After | Gate |
| ------ | ------ | ----- | ---- |
| `ping()` on 10k row KV table | N/A | < 10ms | Hard |
| `is_empty()` on 10k rows | ~count time | < 10ms | Hard |
| `keys_like('%-metadata')` vs `keys()` | O(N) all keys | O(docs) only | Soft (memory test) |
| `count()` accuracy | exact N | exact N | **Must not change** |
| Health handler semantics | is_ok on count | is_ok on ping | Same boolean for healthy DB |

---

## Rollback Plan

Each phase is independently revertable:
1. Phase 1: Revert health.rs + trait ping defaults
2. Phase 2: Revert handler keys_like → keys()
3. Phase 3: Revert upsert to loop
4. Phase 4: Revert to separate pools

No schema migrations required — zero database rollback risk.
