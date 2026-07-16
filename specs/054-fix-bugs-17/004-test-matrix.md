# 004 — Test matrix (correctness + performance)

## 1. Always-on contracts (no Postgres)

| Test | Asserts |
| --- | --- |
| `edgequake-storage` `search_tuning` unit tests | iterative_scan on filtered HNSW/IVFFlat; version gate ≥0.8; ef_search clamp |
| `edgequake-storage/tests/contract_spec054_query_postgres_perf.rs` | Wiring: modes (incl. hybrid/mix)→`query_filtered`; M083 skip-if-exists; lifecycle skip-if-valid; UNIQUE names; stats count path |
| `tests/contract_spec047_p7ef_graph_upsert.rs` | Native upsert SSOT + chunk sizes |
| `edgequake-api/tests/spec045_ingestion_reliability.rs` | Boot ordering; manual resume default; stampede cap |
| `edgequake-query` mode contracts | Fusion / Mix vs Hybrid ordering (memory) |

## 2. Postgres-gated

| Test / script | Asserts | Trigger |
| --- | --- | --- |
| `edgequake-api/tests/e2e_spec054_query_perf_smoke.rs` | M083 apply &lt; 2s when UNIQUE exists; UNIQUE names present | `DATABASE_URL` |
| `edgequake-storage/tests/e2e_spec054_age_pgvector_perf.rs` | Q1–Q3; L1-a AGE batched prefix counts (&lt;200ms); EXPLAIN Index Scan | `--features postgres` + `DATABASE_URL` |
| `edgequake-api/tests/e2e_spec054_documents_list_perf.rs` | L1-a-api in-process documents list (&lt;500ms) + batched AGE reconcile | `DATABASE_URL` |
| `edgequake-storage/tests/e2e_spec054_mix_scale_perf.rs` | Q1-d Mix p95 @50k+ vectors (ex-LLM) | nightly + `DATABASE_URL` |
| `performance_storage.rs` (postgres section) | KV count via stats &lt; 50ms @5k | `DATABASE_URL` |
| `migration_bootstrap_proof.rs` | 038/readiness idempotence | `DATABASE_URL` |
| `backend_e2e_contract.rs` / vector e2e | ANN + filtered query correctness | postgres feature / URL |
| `e2e_spec054_pending_task_reconcile.rs` | Ingest resume/reconcile (not ANN) | postgres |
| `e2e_spec054_pdf_progress_identity.rs` | Progress identity | postgres |

## 3. Informational only (not release gates)

| Artifact | Limitation |
| --- | --- |
| `edgequake/benches/graph_performance.rs` | MemoryGraphStorage |
| `edgequake/benches/BASELINES.md` | Dated; in-memory |
| `tools/bench047/` | RAG quality/fidelity, not DB p95 |

## 4. Gaps still open (tracked)

| Gap | Severity | Proposed gate |
| --- | --- | --- |
| INVALID UNIQUE rebuild path | Medium | unit with mocked pg_index |
| Dual bootstrap race (ensure_indexes + M083) | Low | soak on large graph |

## 5. How to run

```bash
# Contracts (fast)
cd edgequake
cargo test -p edgequake-storage --test contract_spec054_query_postgres_perf
cargo test -p edgequake-storage --lib adapters::postgres::vector::search_tuning
cargo test -p edgequake-api --test spec045_ingestion_reliability

# Postgres smoke + AGE/pgvector budgets (needs make postgres / make dev DB)
export DATABASE_URL="$(cat /tmp/edgequake-db-url)"
cargo test -p edgequake-api --test e2e_spec054_query_perf_smoke -- --nocapture
cargo test -p edgequake-storage --features postgres --test e2e_spec054_age_pgvector_perf -- --nocapture

# Checksums (must stay green — do not touch locked 083 sqlx file)
./scripts/check_migration_checksums.sh
```

## 6. Pass criteria for “perf studied” (this pack)

- [x] First Principles + cross-ref written under `specs/054-fix-bugs-17/`
- [x] Filtered ANN iterative_scan wired + unit-tested
- [x] Boot skip-if-UNIQUE-valid wired in lifecycle + `support/083`
- [x] Contract test locks the wiring
- [x] Postgres B1-a M083 fast-path (~65ms)
- [x] Postgres Q1/Q2/Q3 e2e with tight budgets + EXPLAIN Index Scan
- [x] Native graph writes **default ON** (opt-out `=0`)
- [x] Complexity catalog + July 2026 alignment docs
- [x] Batched AGE source-prefix counts (no list N+1)
- [x] Hybrid/Mix wiring in contracts
- [x] Nightly Mix p95 on 50k+ vectors (Q1-d)