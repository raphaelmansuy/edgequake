# SPEC-022 — EdgeQuake Ingestion & Query Pipeline Study

> **Date**: 2026-06-27  
> **Method**: Code Is Law + First Principles + LightRAG parity lens  
> **Scope**: Full ingestion path (HTTP → pipeline → persistence) and query path (HTTP → engine → retrieval → generation)  
> **Relation to SPEC-021**: Builds on plan-19 closure (`25-brutal-post-closure-assessment.md`). This study re-audits **post-closure** code and surfaces **new** regressions and deferred gaps.

---

## Document map (cross-reference index)

| # | File | Purpose | Key verdict |
|---|------|---------|-------------|
| 00 | [00-executive-brutal-audit.md](./00-executive-brutal-audit.md) | Executive summary, grades, ship/no-ship | **Conditional ship** — async path A; sync upload path **F** |
| 01 | [01-ingestion-pipeline-code-audit.md](./01-ingestion-pipeline-code-audit.md) | Entry points, saga, merger, DRY violations | **3 persistence paths** (should be 1) |
| 02 | [02-query-pipeline-code-audit.md](./02-query-pipeline-code-audit.md) | Modes, retrieval, caches, HTTP parity | Engine **A−**; orchestrator/API parity gap |
| 03 | [03-storage-postgres-age-pgvector.md](./03-storage-postgres-age-pgvector.md) | Version pins, official docs, O(n) storage ops | pgvector **0.7.4** blocks iterative scan |
| 04 | [04-first-principles-solid-dry-on.md](./04-first-principles-solid-dry-on.md) | SOLID, DRY, O(n) contracts | Identity + persister **good**; upload handlers **violate DIP** |
| 05 | [05-cross-reference-index.md](./05-cross-reference-index.md) | Finding ↔ code ↔ test ↔ spec-021 traceability | RC-022-* registry |
| 06 | [06-improvement-plan.md](./06-improvement-plan.md) | Phased remediation P-H1..P-H8 | **P-H1 is blocking** |

---

## Stack versions (Code Is Law)

| Component | Pinned version | Source |
|-----------|----------------|--------|
| PostgreSQL | **16** (bookworm) | `edgequake/docker/Dockerfile.postgres:2` |
| pgvector | **v0.7.4** | `edgequake/docker/Dockerfile.postgres:18` |
| Apache AGE | **PG16/v1.6.0-rc0** | `edgequake/docker/Dockerfile.postgres:26` |
| sqlx | **0.8** | `edgequake/Cargo.toml:123` |
| EdgeQuake | **0.12.11** | `edgequake/Cargo.toml:75` |

---

## Architecture at a glance

```
                    INGESTION (3 paths — problem)
                    =============================

  POST /documents/text ──► task queue ──► text_insert ──┐
  EdgeQuake::insert()  ──► orchestrator ─────────────────┼──► IngestionPersister ✅
                                                          │         │
  POST /documents/upload (sync) ──► file_upload.rs ──────┤         ├──► vectors (batch)
  POST /documents/upload/batch ──► batch_upload.rs ──────┘         └──► merger (batch+saga)

                    (file/batch: inline N+1 upsert ❌ — no merger, no saga)


                    QUERY (mostly unified)
                    ======================

  POST /query ──► query_bootstrap engine ──► QueryEngine ──► 6 modes
                     │ BM25 reranker
                     │ embedding cache
                     └── result cache

  EdgeQuake::query() ──► default engine ──► same QueryEngine
                           │ embedding cache ✅
                           │ result cache ✅
                           └── NO BM25 reranker ⚠️
```

---

## Official documentation (version-aligned)

| Technology | Version used | Primary docs |
|------------|--------------|--------------|
| PostgreSQL 16 | 16-bookworm | [PostgreSQL 16 Documentation](https://www.postgresql.org/docs/16/index.html) |
| pgvector | 0.7.4 | [pgvector README (v0.7.4 tag)](https://github.com/pgvector/pgvector/tree/v0.7.4) — HNSW/IVFFlat, `<=>` cosine ops |
| pgvector iterative scan | **requires ≥0.8.0** | [pgvector 0.8.0 release notes](https://www.postgresql.org/about/news/pgvector-080-released-2952/) |
| Apache AGE | 1.6.0-rc0 (PG16) | [AGE master manual](https://age.apache.org/age-manual/master/) · [Cypher format](https://age.apache.org/age-manual/master/intro/cypher.html) · [PG16/v1.6.0-rc0 release](https://github.com/apache/age/releases/tag/PG16%2Fv1.6.0-rc0) |
| OpenAI embeddings (typical prod) | text-embedding-3-small (1536-d) | [Embeddings guide](https://platform.openai.com/docs/guides/embeddings) |

---

## How to use this study

1. Read **00** for the brutal headline and grades.
2. If fixing ingestion, start at **01** + **06 P-H1**.
3. If fixing query parity, start at **02** + **06 P-H4**.
4. If upgrading infra, start at **03** + **06 P-H3**.
5. Use **05** to trace each finding to tests and prior SPEC-021 work.
