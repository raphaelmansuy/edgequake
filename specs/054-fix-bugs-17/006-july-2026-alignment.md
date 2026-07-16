# 006 — July 2026 alignment (PG16/17/18 · AGE · pgvector)

Checklist against current upstream guidance (researched **2026-07-16**).
Cross-ref: [001-first-principles](./001-first-principles.md), [005-complexity-catalog](./005-query-complexity-catalog.md).

---

## 1. Platform pins

| Component | EdgeQuake images | July 2026 target | Notes |
| --- | --- | --- | --- |
| PostgreSQL | 16 / 17 / 18 Dockerfiles | Keep matrix | PG18: async I/O + B-tree skip scan — free wins, validate on `pg18` tag |
| pgvector | historically 0.8.3 in PG18 image | **≥0.8.2** floor; prefer **0.8.5** | CVE-2026-3172 in 0.8.0/0.8.1 parallel HNSW build |
| Apache AGE | 1.7.0 (PG17/PG18 tags) | 1.7+ / 1.8 PG18 | Official support PG11–18; no indexes by default |

Docker SSOT: `edgequake/docker/Dockerfile.postgres{,.pg17,.pg18}` + `extension-pins.sh`.

---

## 2. PostgreSQL 16 / 17 / 18

| Practice | EdgeQuake | Status |
| --- | --- | --- |
| Index/query design before GUCs | Spec 054 F1–F4 | Aligned |
| `EXPLAIN (ANALYZE, BUFFERS)` gates | Q3-c e2e Index Scan | Aligned |
| Connection pool ≥ peak holders | default `max_connections=32` | Aligned |
| SSD `random_page_cost` | docker images | Keep |
| PG18 async I/O | host PG18 | No app change; matrix CI |
| PG18 skip scan on composite btree | tenant/ws indexes | Planner-automatic |
| Partitioning at 100M+ rows | not yet | Revisit when corpus grows |

**Do not:** `SET enable_seqscan=off` in prod; index every JSONB key; tune memory before indexes.

---

## 3. pgvector 0.8.x

| Practice (upstream / AWS / 2026 guides) | EdgeQuake | Status |
| --- | --- | --- |
| HNSW default for continuous ingest | `VectorIndexType::HNSW` | Aligned |
| `m=16` | default | Aligned |
| `ef_construction` 64–128 prod start | default **32** (SPEC-034 size tradeoff) | **Profile:** env `EDGEQUAKE_HNSW_EF_CONSTRUCTION` (32 dev / 128 prod); no boot REINDEX |
| Filtered ANN → `iterative_scan` | `relaxed_order` + `max_scan_tuples=20000` | Aligned |
| Unfiltered ANN → iterative off | `query()` does not set it | Aligned |
| `ef_search = f(top_k)` | `clamp(4×top_k, 40, 1000)` | Aligned |
| halfvec for dim > 2000 | `AnnIndexPolicy` | Aligned |
| Fail-closed ANN DDL | `ddl.rs` | Aligned |
| Drop unused metadata GIN | M073 | Aligned |
| Extension floor ≥0.8.2 | capabilities readiness | Enforced in code |

**Operator REINDEX (production bump of ef_construction):**

```sql
-- After setting EDGEQUAKE_HNSW_EF_CONSTRUCTION=128 for new indexes:
-- REINDEX INDEX CONCURRENTLY eq_<ns>_vectors_embedding_idx;
-- (or DROP + CREATE with new WITH (...); never on listen-critical path)
```

---

## 4. Apache AGE

| Practice (Microsoft Learn AGE performance) | EdgeQuake | Status |
| --- | --- | --- |
| AGE creates **no** indexes by default | `ensure_indexes` + M038/M083 | Aligned |
| BTREE on id / start_id / end_id | lifecycle | Aligned |
| GIN on properties + expr BTREE hot keys | GIN + UNIQUE node_id / edge ends | Aligned |
| EXPLAIN inside Cypher for plans | Q3-c style gates | Aligned |
| Prefer SQL for aggregation / ID lookup | native upsert + batch get | Aligned |
| Explicit labels in MATCH | `:Node` / `:EDGE` | Aligned |
| Cypher property match ≠ index guarantee | F2 / AGE#2348 | Documented; native path default |

**Production write path:** `EDGEQUAKE_NATIVE_GRAPH_WRITES` default ON. Cypher MERGE = debug fallback only.

---

## 5. Complexity contract (query modes)

| Mode | Vector | Graph | Allowed |
| --- | --- | --- | --- |
| Local / Global / Naive / Chunk | `query_filtered` | batch expand | OK |
| Hybrid / Mix | `query_filtered` both arms | batch expand | OK (contract-locked) |
| Community | bounded load | not full-scan | OK |
| Documents list reconcile | — | **batched** `node_counts_by_source_prefixes` | OK |
| Boot M083 | — | skip if UNIQUE valid | OK |

---

## 6. Gaps closed by this remediation pack

| Gap | Fix |
| --- | --- |
| Documents list N+1 AGE prefix counts | Batched analytics API |
| Hybrid/Mix not in wiring contract | `contract_spec054` includes hybrid/mix |
| Spec 054 e2es not in CI | Nightly / quality-gates wiring |
| L1-a list latency manual only | Automated e2e when DB warm |
| Q1-d Mix @50k+ | Nightly scale test |
| Escape helper duplication | Single `escape` module |
| Unbound Cypher on writes | Bound `$1` for new write paths |

---

## 7. References (fetched 2026-07)

- [pgvector README / v0.8.5](https://github.com/pgvector/pgvector) — iterative scans, HNSW params
- [pgxn vector 0.8.5](https://pgxn.org/dist/vector/0.8.5/)
- [AWS: pgvector 0.8 iterative scans](https://aws.amazon.com/blogs/database/supercharging-vector-search-performance-and-relevance-with-pgvector-0-8-0-on-amazon-aurora-postgresql/)
- [Microsoft: Apache AGE performance](https://learn.microsoft.com/en-us/azure/postgresql/azure-ai/generative-ai-age-performance)
- [Apache AGE README](https://github.com/apache/age) — PG11–18 support
- PostgreSQL 18: async I/O, B-tree skip scan (community 2026 guides)
