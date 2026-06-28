# 008 — Postgres / AGE / pgvector Expert Lens

**Cross-ref:** [005 Features](../005-features/001-feature-matrix.md) · [006 Robustness](../006-robustness/001-robustness-comparison.md)

**Finding:** C-04, C-05

---

## Storage Stack Comparison

### LightRAG Postgres (`kg/postgres_impl.py`)

```text
  PostgreSQL
  ├── KV tables (JSONB)
  ├── Vector tables (pgvector)
  ├── Graph (via Cypher/SQL — varies)
  └── Doc status tables

  + 12 other backend options (Neo4j, Milvus, Qdrant, ...)
```

Postgres is **one option among many**. Tests exist (`tests/kg/postgres_impl/`) but it's not the default dev path.

### EdgeQuake Postgres (mandatory)

```text
  PostgreSQL 16+
  ├── Apache AGE (graph — Cypher)
  ├── pgvector (HNSW indexes)
  ├── KV tables (chunks, metadata, hashes)
  ├── Workspace registry
  ├── Task queue (JSONB payload)
  └── FTS (tsvector on chunk content)

  No fallback. DATABASE_URL required.
```

**Source:** `edgequake-storage/src/adapters/postgres/`, `state/postgres.rs`.

---

## pgvector Usage

| Pattern | LightRAG PG | EdgeQuake |
|---------|:-----------:|:---------:|
| Chunk embeddings | ✓ | ✓ `content_ref` metadata |
| Entity embeddings | ✓ | ✓ separate collection |
| Relationship embeddings | ✓ | ✓ separate collection |
| HNSW index | △ impl-dep | ✓ |
| Batch UNNEST upsert | △ | ✓ QW2 single transaction |
| Workspace filter | △ | ✓ tenant+workspace |
| Deferred embedding | ✓ test | △ |

EdgeQuake chunk vectors store **`content_ref`** (KV key) not inline text — **storage SSOT** LightRAG doesn't enforce uniformly.

---

## Apache AGE Graph

| Operation | LightRAG (NetworkX default) | EdgeQuake AGE |
|-----------|:-------------------------:|:-------------:|
| Node upsert | in-memory | Cypher MERGE |
| Edge upsert | in-memory | Cypher MERGE |
| BFS / hops | O(V+E) memory | batch Cypher + `get_incident_edges_batch` |
| Source tracking | ✓ source_ids | ✓ source_chunk_ids |
| Cascade delete | ✓ | ✓ document_graph_cascade |
| Community property | ✗ | ✓ community_id |

**EdgeQuake AGE integration is production-grade.** LightRAG NetworkX default is not.

---

## Full-Text Search

```text
  LightRAG:  no native FTS in reference query path
  EdgeQuake:  Postgres tsvector + GIN on chunk content
              fused with vector scores in chunk_retrieval.rs
```

EdgeQuake **exceeds** LightRAG Postgres impl on hybrid retrieval.

---

## SQL Patterns (EdgeQuake strengths)

| Pattern | File | Why it matters |
|---------|------|----------------|
| Chunked UNNEST vector insert | `postgres/vector/` | Atomic per-doc write |
| Metadata filter SQL builder | `metadata_filter_sql.rs` | Safe tenant scoping |
| Community ID push-down | `graph/scan_ops.rs` | O(communities) not O(graph) |
| Workspace hash keys | `storage_helpers.rs` | KV isolation |

---

## Migration & Schema

| Concern | LightRAG | EdgeQuake |
|---------|:--------:|:---------:|
| Auto migration | △ storage_migrations | ✓ migration_bootstrap |
| Version tracking | △ | ✓ m038, m042, etc. |
| Reconcile on startup | △ | ✓ reconcile modules |

---

## Postgres Expert Verdict

| Dimension | LightRAG PG | EdgeQuake PG |
|-----------|:-----------:|:------------:|
| pgvector design | **B** | **A** |
| AGE graph ops | **N/A default** | **A-** |
| FTS integration | **F** | **A-** |
| Multi-tenant SQL | **B-** | **A** |
| Transaction boundaries | **B** | **A-** (saga) |
| Backend portability | **A** (13 opts) | **F** (1 stack) |

**EdgeQuake wins decisively on Postgres-native RAG.**

LightRAG wins if you **need Neo4j or Milvus** — EdgeQuake won't help.

---

## Risk: AGE Maturity

Both depend on Apache AGE (EdgeQuake exclusively):

- AGE Cypher dialect ≠ Neo4j fully
- Complex graph analytics limited vs dedicated graph DB
- EdgeQuake mitigates with batch ops + property indexes

**Acceptable trade** for RAG workloads (entity lookup + 2-hop BFS), not for graph analytics platform.
