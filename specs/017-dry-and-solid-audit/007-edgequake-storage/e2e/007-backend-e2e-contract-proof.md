# P2-11 — Cross-backend E2E contracts

**Date:** 2026-06-03  
**Status:** ✅ Proven (memory); postgres when `POSTGRES_PASSWORD` set

## Change

Shared assertion modules under `tests/support/`:

| Module | Assertions |
|--------|------------|
| `kv_e2e_contract.rs` | basic CRUD, bulk, filter_keys |
| `vector_e2e_contract.rs` | basic CRUD, query, bulk count |
| `graph_e2e_contract.rs` | node CRUD, edge CRUD, hub edges |

`backend_e2e_contract.rs` runs identical contracts on memory (10 tests) and postgres (8 tests) via macros.

`e2e_storage_backends::postgres_tests` now delegates to the same contracts (DRY).

## Proof

```bash
cargo test -p edgequake-storage --test backend_e2e_contract
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh
```

Postgres parity:

```bash
POSTGRES_PASSWORD=... ./specs/.../run_storage_e2e.sh --with-postgres
```

## SOLID

- **DRY:** One assertion per behavior; memory/postgres differ only in factory.
- **OCP:** New backend = new factory + same contract calls.
- **LSP:** Contracts require trait semantic parity, not implementation details.
