# P1 — Shared query bootstrap (API-DRY-003)

**Status:** ✅ Fixed  
**Date:** 2026-06-04

## Claim

Memory and PostgreSQL `AppState` bootstraps share query-engine and ingestion pipeline construction via `state/query_bootstrap.rs`.

## Evidence

```bash
cargo test -p edgequake-api --test spec017_api_contract spec017_shared_query_bootstrap
```

| Function | Used by |
|----------|---------|
| `build_ingestion_pipeline` | `memory.rs`, `postgres.rs` |
| `build_production_query_engines` | `memory.rs`, `postgres.rs` |

Eliminates duplicated `QueryEngine::new` + `SOTAQueryEngine::new` + `Pipeline::default_pipeline().with_extractor(...)` blocks.
