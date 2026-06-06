# E2E Proof 017 — P6 Community Seal + Readiness Battle Test

**Requirement:** NFR-006-003  
**Status:** ✅ Verified 2026-06-06

---

## Claim

Unguarded full-graph community detection cannot be imported from `edgequake_storage` crate root; `/ready` returns 503 when migration 038 indexes are degraded.

---

## Evidence

### Storage seal (compile-time boundary)

- `detect_communities` renamed → `detect_communities_unchecked` in `community.rs`
- Removed from `edgequake_storage::lib.rs` re-exports
- Only `graph_community.rs` may call `detect_communities_unchecked`

```bash
./scripts/spec006_no_unguarded_community_api.sh
```

### Readiness battle test

```bash
cargo test -p edgequake-api --test migration_readiness_proof --features postgres
```

Cases:
- `OK` when no bootstrap report (memory/test state)
- `503` when `migration_038.indexes_ready == false` and AGE available
- `OK` when indexes ready

### Postgres battle suite

```bash
make resource-proof-postgres
```

Runs `test-postgres-start` + `migration_bootstrap_proof` + full `resource-proof`.

---

## Regression

Included in `make resource-proof` (readiness tests always; bootstrap e2e when `DATABASE_URL` set).
