# Implementation Proof — SPEC-011 Storage Performance (updated)

> **Phase 1**: ping, batch upsert, shared pool, keys_like on list/stats  
> **Phase 2**: brutal assessment, guarantee model, keys_with_prefix, E2E SLO tests

---

## Brutal summary

| Claim | True? |
| ----- | ----- |
| Fixed 13s health COUNT | **Yes** — proven by SLO test |
| Made all storage fast | **No** — suffix LIKE still O(N) in PG |
| Guaranteed production perf | **Partial** — G1 ops only; CI on memory backend |

See [BRUTAL_ASSESSMENT.md](./BRUTAL_ASSESSMENT.md).

---

## Phase 2 additions

| Change | Files |
| ------ | ----- |
| `keys_with_prefix()` | `traits/kv.rs`, postgres + memory adapters |
| Migrated callers (no full `keys()`) | detail, impact, bulk, lineage, costs×2, storage_helpers, pipeline_checkpoint |
| Memory `ping` / filtered key scans | `adapters/memory/kv.rs` |
| E2E SLO tests (7 tests, 2600 KV rows) | `e2e_storage_performance_spec011.rs` |
| Guarantee documentation | `PERFORMANCE_GUARANTEE.md` |

---

## Phase 3 — O(1) `count()` (eliminates 13s COUNT query even when called)

| Change | Detail |
| ------ | ------ |
| `{prefix}_kv_stats` table | Single-row `row_count` counter |
| INSERT/DELETE triggers | Maintain exact count on upsert/delete |
| `count()` SQL | `SELECT row_count FROM …_kv_stats` — **not** `SELECT COUNT(*) FROM …_kv` |
| `clear()` | `TRUNCATE` + reset stats to 0 |
| Regression test | `test_health_handler_never_calls_kv_count` + `test_postgres_kv_count_is_o1_via_stats_table` |

See [COUNT_QUERY_FIX.md](./COUNT_QUERY_FIX.md).

---

## Test commands & results

```bash
# G1 contract tests (must pass in CI)
cargo test -p edgequake-api --test e2e_storage_performance_spec011
# 7 passed — health <200ms, ping <50ms, prefix scan <100ms, list <500ms @ 2600 rows

cargo test -p edgequake-storage --test performance_storage --features postgres
# 7 passed (6 memory + 1 postgres skip without POSTGRES_PASSWORD)

cargo test -p edgequake-api --test e2e_dashboard_stats_issue81
# 14 passed — KPI semantics unchanged

cargo test -p edgequake-storage --test e2e_storage_backends
# 35 passed

cargo clippy -p edgequake-storage -p edgequake-api --features postgres -- -D warnings
# clean
```

---

## SLO table (enforced in E2E)

| Test | Threshold | Load |
| ---- | --------- | ---- |
| `test_health_slo_with_large_kv` | < 200 ms | 2600 KV rows |
| `test_kv_ping_slo_with_large_kv` | < 50 ms | 2600 KV rows |
| `test_keys_with_prefix_slo_and_correctness` | < 100 ms, 25 keys | single doc |
| `test_document_list_slo_with_large_kv` | < 500 ms | 100 docs |
| `test_document_detail_uses_prefix_not_full_scan` | < 100 ms | single doc |
| `test_count_still_exact_under_load` | exact 2600 | semantics |

---

## Remaining `keys()` call sites (not G1)

Still unbounded — track for phase 3:

- `main.rs`, `orchestrator/deletion.rs`, `pdf_processing.rs`
- `injection.rs`, `tasks.rs`, `workspace_crud.rs`
- `recovery/stuck.rs`, `recovery/reprocess.rs`, `delete/single.rs`

---

## Cross-references

- [PERFORMANCE_GUARANTEE.md](./PERFORMANCE_GUARANTEE.md) — how guarantees work
- [BRUTAL_ASSESSMENT.md](./BRUTAL_ASSESSMENT.md) — honest limits
- [IMPROVEMENT_PLAN.md](./IMPROVEMENT_PLAN.md) — phase 3 backlog
