# edgequake-storage — DRY & SOLID Audit

**Crate path:** `edgequake/crates/edgequake-storage`  
**LOC:** ~15,100 (src)  
**Last verified:** 2026-06-03T08:28:24Z — query `run_query_e2e.sh` + Playwright `01–02` re-capture; no storage source changes; storage memory e2e PASS (prior run); PNGs `05–08` (storage) + `01–02` (query) on disk

---

## Executive Summary

**P0–P3 remediation is complete and test-proven** on memory and PostgreSQL contract paths.

- **Memory:** `./e2e/run_storage_e2e.sh` → PASSED (`001-test-run.log`)
- **PostgreSQL:** `./e2e/run_storage_e2e.sh --with-postgres` → PASSED (auto-detects port from `/tmp/edgequake-db-url`)
- **UI:** `./e2e/run_playwright_proof.sh` → 2 specs, PNGs `05–08`

**ISP:** Phase 1 ✅ · Phase 2a (`GraphReadView`) ✅ · Phase 2b (op traits) ✅

**API DRY:** `postgres_user_bootstrap.rs` (anonymous user upsert for chat + conversations).

**Test DRY (this session):** `postgres_test_config.rs` — pool builder + `seed_tenant_and_user`; conversation contract uses `assert_conversation_crud_contract_with_ids` for FK-safe postgres runs.

---

## Remediation Status

| ID | P | Item | Status | Evidence |
|----|---|------|--------|----------|
| P0-1–P0-3 | P0 | Workspace counts, contracts, dashboard PNGs | ✅ | PNGs 05–06 |
| P1-4–P1-9 | P1 | MetadataFilter, PDF, conversation, API wiring | ✅ | |
| P2-10–P2-11 | P2 | Backend E2E contracts | ✅ | `backend_e2e_contract` |
| P1-12 / P1-13 | P1 | Graph helpers + PgVector split | ✅ | `005`, `010` proofs |
| P2-16 / P2-17 | P2 | Graph batch + workspace cache | ✅ | `006` proof |
| P3-18 / P3-21 | P3 | Conversation HTTP + Playwright | ✅ | `008`, `011`, `015`, PNGs 07–08 |
| P3-19 / 19b / 19c | P3 | GraphStorage ISP | ✅ | `009`, `012`, `014`, 7 ISP tests (6 memory + 1 postgres) |
| P2-22 | P2 | e2e_storage_backends | ✅ | 35 tests, `013` |
| P1-20 | P1 | Postgres CI gate | ✅ | `postgres-integration.yml` |
| P3-23 | P3 | API user bootstrap DRY | ✅ | `postgres_user_bootstrap.rs` |
| P1-24 | P1 | Postgres test `#[path]` fix | ✅ | hoisted `postgres_test_config` |
| P1-25 | P1 | Runner postgres port auto-detect + `make db-start` if no URL file | ✅ | `run_storage_e2e.sh` |
| P1-26 | P1 | Postgres conversation FK seed | ✅ | `seed_tenant_and_user` |
| P2-27 | P2 | Contract support clippy (GraphStorage bound) | ✅ | `graph_*_contract.rs` use supertrait only |

---

## DRY / SOLID (summary)

| ID | Status |
|----|--------|
| STORE-DRY-001–004 | ✅ |
| STORE-DRY-003 e2e_storage_backends | ✅ |
| STORE-SOLID-S-002 / S-003 | ✅ |
| STORE-SOLID-I-001 GraphStorage ISP | ✅ 1, 2a, 2b |
| API-DRY-001 anonymous user upsert | ✅ |
| TEST-DRY-001 postgres contract env | ✅ `postgres_test_config.rs` |

---

## Brutal Assessment

### Proven (code is law)

**Prerequisite for postgres:** `make db-start` (or prior `make dev-bg`) so `/tmp/edgequake-db-url` exists.

```bash
make db-start   # if not already running
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_storage_e2e.sh --with-postgres
./specs/017-dry-and-solid-audit/007-edgequake-storage/e2e/run_playwright_proof.sh

cd edgequake && cargo check --workspace
cargo test -p edgequake-storage --test e2e_storage_backends      # 35
cargo test -p edgequake-storage --test graph_isp_contract       # 6 memory; +1 postgres with --features postgres
cargo test -p edgequake-api --test spec017_conversation_http_contract  # 2
```

| Claim | Proof | Gap |
|-------|-------|-----|
| Conversation trait (memory) | `conversation_backend_contract` | ✅ |
| Conversation trait (postgres) | same + `seed_tenant_and_user` | ✅ this session |
| Conversation HTTP (memory app) | `spec017_conversation_http_contract` | ✅ not live Axum integration test |
| Conversation UI + PG | Playwright + 07–08 | ✅ |
| Dashboard stats UI | 05–06 | ✅ |
| Memory parity | `backend_e2e_contract`, `e2e_storage_backends` | ✅ |
| Postgres parity | `--with-postgres` runner (17 postgres backend tests) | ✅ |
| ISP 2b | `graph_isp_contract` (7 total with postgres feature) | ✅ |
| POST /conversations FK | API bootstrap + Playwright | ✅ |

### On-disk E2E inventory (actual)

| Path | Present |
|------|---------|
| `e2e/run_storage_e2e.sh` | ✅ |
| `e2e/run_playwright_proof.sh` | ✅ |
| `e2e/001-test-run.log` | ✅ |
| `e2e/001`–`003` P0–P1 proofs | ✅ |
| `e2e/004` Playwright dashboard | ✅ |
| `e2e/005`–`015` narrative proofs | ✅ |
| `e2e/screenshots/05–08.png` | ✅ (4 files, refreshed 07:17 UTC) |

### Cross-crate (edgequake-query, 2026-06-03)

| Impact on storage | Detail |
|-------------------|--------|
| Retrieval API | Query crate now always calls `VectorStorage::query_filtered` / graph batch ops via unified `vector_queries` — **no storage trait changes** this session |
| Hybrid behavior | Default API path hybrid now matches workspace (local + global + naive); more vector round-trips per query — monitor postgres load |
| Proof | `run_query_e2e.sh` + API `spec017_query_production_path_contract` (2) + Playwright `01–02`; storage memory runner re-PASS 2026-06-03T08:14Z — re-run `--with-postgres` before release |

### Not fixed (honest gaps)

1. **`query_ops.rs` / `nodes_ops.rs`** — large modules; acceptable.
2. **`memory/vector.rs`** — adapter MetadataFilter tests; complements `metadata_filter_dry_contract`.
3. **Phase 2c** — optional `GraphReadView` at more read-only call sites.
4. **edgequake_webui** — “New conversation” button does not call create API (Playwright proves storage via direct POST + history list).
5. **Live Axum conversation HTTP** — only memory `TestServer` contract; production path covered by Playwright on real stack.
6. **Query crate** — retrieval unified on `vector_queries`; triple batch embed when keywords off (`005-embedding-triple-batch-proof.md`). Re-run `run_storage_e2e.sh --with-postgres` before release. See `006-edgequake-query/001-audit.md`.

---

## E2E Artifacts

| File | Type |
|------|------|
| `e2e/run_storage_e2e.sh` | Rust runner (memory + optional postgres) |
| `e2e/run_playwright_proof.sh` | Playwright + stack |
| `e2e/screenshots/05–08*.png` | Visual proof |
| `e2e/001`–`015-*-proof.md` | Traceability narratives |

---

## Next Steps

### P2 — polish
1. Phase 2c: `GraphReadView` / `dyn GraphStorageReadOps` where full graph arc is unnecessary.
2. Crate rustdoc for `GraphStorage*Ops` on concrete types.

### P3 — product (webui)
3. Wire history “New conversation” to `useCreateConversation`.
4. Query UI PNGs `01–02` captured (2026-06-03); storage PNGs `05–08` unchanged.

### P1 — CI
5. Keep `postgres-integration.yml` SPEC-017 storage job green on push.
6. Re-run `./e2e/run_storage_e2e.sh --with-postgres` after query merge to confirm no adapter regressions.
