# P2 — Graph batch upsert + workspace vector cache DRY

**Date:** 2026-06-03  
**Status:** ✅ Proven (memory); postgres when `POSTGRES_PASSWORD` set

## Changes

### Graph batch contract (audit item 6)
- `tests/support/graph_batch_contract.rs` — shared `assert_graph_batch_upsert`
- Wired into `storage_backend_contract.rs` for memory + postgres

### Workspace vector cache (STORE-DRY-004)
- `src/adapters/workspace_vector_cache.rs` — double-checked `get_or_create` with validation hook
- `MemoryWorkspaceVectorRegistry` and `PgWorkspaceVectorRegistry` delegate to shared cache

### E2E fixtures DRY (P2-11 partial)
- `tests/support/postgres_test_config.rs` — single postgres config builder
- `tests/support/e2e_fixtures.rs` — namespace + graph property helpers
- `e2e_storage_backends.rs`, `conversation_backend_contract.rs` use shared modules

## Proof

```bash
cd edgequake && cargo check --workspace
cd edgequake && cargo test -p edgequake-storage --test storage_backend_contract memory_backend_graph_batch
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh
```

Postgres batch + workspace stats:

```bash
POSTGRES_PASSWORD=... ./specs/.../run_storage_e2e.sh --with-postgres
```

## SOLID mapping

- **DRY:** One cache implementation, one postgres config, one batch contract.
- **SRP:** Cache module owns locking only; registries own backend-specific create logic.
- **OCP:** Postgres dimension validation plugs in via `validate` closure without changing cache.
