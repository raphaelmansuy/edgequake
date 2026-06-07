# P1-13 / P3-15 — PgVectorStorage SRP + constructor dedup

**Date:** 2026-06-03  
**Status:** ✅ Proven

## Change

Monolithic `adapters/postgres/vector/mod.rs` (~686 LOC) split into focused modules:

| Module | Responsibility | ~LOC |
|--------|----------------|------|
| `mod.rs` | Struct, `from_parts` (STORE-P3-15), public factories, Debug | 111 |
| `ddl.rs` | Table create/drop, stats row counter | 151 |
| `migration.rs` | Dimension detection + migration guard | 121 |
| `search_tuning.rs` | Embedding format/parse, iterative scan tuning | 184 |
| `storage_impl.rs` | `VectorStorage` trait implementation | 554 |

All public constructors (`new`, `with_pool`, `with_dimension`, `with_pool_and_dimension`) delegate to private `from_parts(pool, config, dimension)`.

## Proof

```bash
cd edgequake && cargo check --workspace
cd edgequake && cargo test -p edgequake-storage --lib --tests
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh
```

Last run: **PASSED** — see `001-test-run.log` (2026-06-03).

## SOLID mapping

- **SRP:** DDL, migration, search tuning, and trait impl are separate compilation units.
- **OCP:** New index/tuning behavior can extend `search_tuning.rs` without touching DDL.
- **DRY:** Single constructor path eliminates duplicated field initialization across four factories.
