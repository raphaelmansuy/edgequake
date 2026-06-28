# COUNT(*) Query Fix — `eq_eq_default_kv`

> **Incident query** (13.753s):
> ```sql
> SELECT COUNT(*) as count FROM public.eq_eq_default_kv
> ```

---

## Two-layer fix (both required)

| Layer | What | Result |
| ----- | ---- | ------ |
| **1. Stop calling it on hot paths** | `/health` uses `ping()` not `count()` | Probe never emits COUNT on KV |
| **2. Make `count()` O(1) when called** | `{table}_stats` row counter + INSERT/DELETE triggers | Even if `count()` is called, no full scan |

---

## Layer 1 — Health / probes

```rust
// health.rs — BEFORE (13s on 100k rows)
kv_storage: state.kv_storage.count().await.is_ok(),

// AFTER (<1ms)
kv_storage: state.kv_storage.ping().await.is_ok(),
```

**Enforcement**: `test_health_handler_never_calls_kv_count` greps `health.rs` source.

---

## Layer 2 — O(1) maintained counter

Tables per namespace:

- `public.eq_{prefix}_kv` — data
- `public.eq_{prefix}_kv_stats` — single row `(id=1, row_count)`

Triggers:

- `AFTER INSERT` → `row_count += 1` (upsert updates do not fire INSERT on conflict)
- `AFTER DELETE` → `row_count -= 1`
- `TRUNCATE` (clear) → stats reset to 0 in application code

```rust
// count() — AFTER
SELECT row_count FROM public.eq_eq_default_kv_stats WHERE id = 1
```

**One-time cost**: first `initialize()` after upgrade runs `INSERT … SELECT COUNT(*) … ON CONFLICT DO NOTHING` to backfill stats (single migration scan).

**Enforcement**: `test_postgres_kv_count_is_o1_via_stats_table` — 5000 rows, `count()` < 50ms.

---

## Query latency expectations

| Query | Before | After |
| ----- | ------ | ----- |
| `SELECT COUNT(*) FROM eq_eq_default_kv` | O(N) 13s+ | **Not emitted** on health |
| `SELECT row_count FROM eq_eq_default_kv_stats` | N/A | O(1) <1ms |
| `SELECT 1 FROM eq_eq_default_kv LIMIT 1` (ping) | N/A | O(1) <1ms |

---

## Cross-references

- [PERFORMANCE_GUARANTEE.md](./PERFORMANCE_GUARANTEE.md)
- [BRUTAL_ASSESSMENT.md](./BRUTAL_ASSESSMENT.md)
- `edgequake-storage/src/adapters/postgres/kv.rs` — implementation
