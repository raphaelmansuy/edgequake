# 05 — Cross-Reference Index (Findings ↔ Code ↔ Tests ↔ SPEC-021)

> **Purpose**: High-signal traceability. One row = one actionable truth.

---

## RC-022 finding registry

| ID | Sev | Title | Primary code | Tests | SPEC-021 relation | Doc |
|----|-----|-------|--------------|-------|-------------------|-----|
| RC-022-1 | CRITICAL | Sync file upload bypasses persister | `file_upload.rs:275-503` | **none** | Regresses P-G2 fix | [01](./01-ingestion-pipeline-code-audit.md) |
| RC-022-2 | HIGH | Batch upload bypass + no graph | `batch_upload.rs:156-211` | `e2e_file_upload.rs` (structure only) | Regresses P-G2 | [01](./01-ingestion-pipeline-code-audit.md) |
| RC-022-3 | HIGH | 4 write semantics, not 2 | entry point table | partial | Contradicts plan-25 "2 paths" | [01](./01-ingestion-pipeline-code-audit.md) |
| RC-022-4 | MED | Orchestrator missing BM25 | `orchestrator/mod.rs:519-528` | `spec021_orchestrator_*` (cache only) | Extends plan-25 §6 | [02](./02-query-pipeline-code-audit.md) |
| RC-022-5 | MED | pgvector 0.7.4 vs 0.8 code path | `Dockerfile.postgres:18`, `search_tuning.rs` | unit in search_tuning | New infra gap | [03](./03-storage-postgres-age-pgvector.md) |
| RC-022-6 | MED | Cypher string build | `nodes_ops.rs`, `cypher_exec.rs` | integration | Known pattern | [03](./03-storage-postgres-age-pgvector.md) |
| RC-022-7 | LOW | O(E) LLM summarization | `ingestion_persister.rs:262` | merger tests | Accepted trade-off | [01](./01-ingestion-pipeline-code-audit.md) |
| RC-022-8 | LOW | No GraphRAG communities | — | — | plan-25 deferred | [02](./02-query-pipeline-code-audit.md) |

---

## SPEC-021 closed items (still true — do not reopen)

| ID | Item | Verified in 022 | Evidence |
|----|------|-----------------|----------|
| RC-6 | EntityId SSOT | ✅ | `entity_id.rs`, merger |
| RC-7 | Single persister (2 callers) | ✅ partial | persister + **upload exception** |
| RC-11 | Query engine consolidation | ✅ | `query_bootstrap.rs` |
| P-G3 | Global N+1 fix | ✅ | `contract_global_no_nplus1.rs` |
| P-G9 | Embedding + result cache | ✅ | `contract_*_cache.rs` |
| P-G1b | Legacy reconcile admin | ✅ | `entity_reconcile.rs` |

---

## Code anchor quick index

### Ingestion

| Symbol | File | Role |
|--------|------|------|
| `IngestionPersister` | `edgequake-pipeline/src/persistence/ingestion_persister.rs` | DIP port |
| `KnowledgeGraphMerger` | `edgequake-pipeline/src/merger/` | dedup + batch |
| `EntityId` | `edgequake-storage/src/entity_id.rs` | identity SSOT |
| `upload_file` | `edgequake-api/src/handlers/documents/upload/file_upload.rs` | **RC-022-1** |
| `process_text_insert` | `edgequake-api/src/processor/text_insert.rs` | canonical async |
| `EdgeQuake::insert` | `edgequake-core/src/orchestrator/ingestion.rs` | SDK canonical |

### Query

| Symbol | File | Role |
|--------|------|------|
| `QueryEngine` | `edgequake-query/src/engine_impl/` | retrieval core |
| `query_local_with_vector_storage` | `vector_queries.rs` | local mode |
| `build_production_query_engine` | `edgequake-api/src/state/query_bootstrap.rs` | API bootstrap |
| `QueryMode` | `edgequake-query/src/modes.rs` | mode enum |

### Storage

| Symbol | File | Role |
|--------|------|------|
| `PgVectorStorage` | `edgequake-storage/.../vector/storage_impl.rs` | pgvector |
| `PostgresAGEGraphStorage` | `edgequake-storage/.../graph/` | AGE |
| `MetadataFilter::build_sql` | `metadata_filter_sql.rs` | SQL filters |
| `pgvector_supports_iterative_scan` | `search_tuning.rs` | version gate |

---

## Test matrix (contract coverage)

```
                        persister  merger  upload  postgres
                        ─────────  ──────  ──────  ────────
contract_ingestion_persistence   ✅      —       —       —
contract_merger_graph_batch      —       ✅      —       —
contract_entity_identity         —       ✅      —       —
contract_global_no_nplus1        —       —       —       —
contract_query_modes             —       —       —       —
e2e_file_upload                  —       —       ⚠️      —
worker + postgres E2E            —       —       ❌      ❌
```

**Legend**: ✅ covered · ⚠️ partial · ❌ missing · — N/A

---

## Dependency version cross-ref

| Artifact | Version | Doc file |
|----------|---------|----------|
| PostgreSQL | 16 | [03 §1](./03-storage-postgres-age-pgvector.md) |
| pgvector | 0.7.4 | [03 §2](./03-storage-postgres-age-pgvector.md) |
| Apache AGE | 1.6.0-rc0 | [03 §3](./03-storage-postgres-age-pgvector.md) |
| sqlx | 0.8 | README |
| edgequake-llm | 0.6.20 | `Cargo.toml` |

---

## Document dependency graph

```
README.md
    │
    ├── 00-executive-brutal-audit.md ──────┐
    │                                       │
    ├── 01-ingestion-pipeline-code-audit ──┼──► 06-improvement-plan.md
    ├── 02-query-pipeline-code-audit ──────┤
    ├── 03-storage-postgres-age-pgvector ──┤
    ├── 04-first-principles-solid-dry-on ──┘
    │
    └── 05-cross-reference-index.md (this file)
```
