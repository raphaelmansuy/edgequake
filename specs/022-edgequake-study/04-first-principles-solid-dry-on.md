# 04 — First Principles: SOLID, DRY, O(n)

> **Cross-ref**: [01-ingestion](./01-ingestion-pipeline-code-audit.md) · [02-query](./02-query-pipeline-code-audit.md) · [05-cross-ref](./05-cross-reference-index.md)

---

## 1. First-principles axioms for EdgeQuake

| # | Axiom | Enforcement mechanism | Status |
|---|-------|----------------------|--------|
| A1 | **One real-world entity → one graph node** | `EntityId` newtype | ✅ merger path; ⚠️ legacy data |
| A2 | **Graph node id and entity vector id are derived from same identity** | `EntityId::as_graph_node_id` / `as_vector_id` | ✅ |
| A3 | **Cross-store writes follow saga, not hope** | `IngestionPersister` | ✅ A,B paths only |
| A4 | **Storage batch APIs are O(1) round-trips per logical batch** | required trait methods | ✅ merger + query |
| A5 | **Query retrieval is bounded by top_k, not graph size** | ANN + batch graph | ✅ |
| A6 | **Every HTTP ingestion route uses the same persistence port** | DIP | **❌ violated by C,D** |

---

## 2. SOLID assessment

### S — Single Responsibility

| Module | Responsibility | Verdict |
|--------|----------------|---------|
| `edgequake-pipeline` | chunk/extract/embed | ✅ clean |
| `ingestion_persister.rs` | cross-store persist + saga | ✅ clean |
| `merger/` | dedup + graph merge | ✅ clean |
| `edgequake-query` | retrieve + generate | ✅ clean |
| `file_upload.rs` | HTTP + **persistence + graph** | **❌ god handler** |

**Violation**: `upload_file` owns HTTP parsing, pipeline invocation, KV writes, vector writes, graph writes, metadata — **5 reasons to change in one file**.

### O — Open/Closed

- Query modes extend via `QueryMode` enum + engine match — closed for extension without code change  
- `IngestionPersister` trait allows alternate persist strategies — **good OCP**  
- Upload handlers **closed to extension** — any persist fix must be copy-pasted until routed through trait

### L — Liskov Substitution

- `GraphStorage` batch methods required (P-G10) — memory adapter must implement real batches  
- `VectorStorage::upsert` batch semantics documented — callers rely on atomicity  
- **Substituting persister with inline loops** breaks saga contract — **LSP violation** in upload path

### I — Interface Segregation

Storage traits split well:

```
GraphStorage = GraphReadOps + GraphMutateOps + GraphAnalyticsOps + ...
Query engine uses graph_read() — read-only view ✅
```

**Good ISP** — query cannot accidentally mutate graph.

### D — Dependency Inversion

**High point of codebase**:

```21:30:edgequake/crates/edgequake-pipeline/src/persistence/ingestion_persister.rs
/// Ingestion persistence port (P-G2d / DIP). Callers depend on this trait, not
/// storage details.
#[async_trait]
pub trait IngestionPersister: Send + Sync { ... }
```

Orchestrator + processor depend on trait ✅  
Upload handlers depend on concrete `GraphStorage` + manual loops ❌

```
        ┌─────────────────────┐
        │ IngestionPersister  │  ◄── orchestrator, text_insert
        └─────────────────────┘
                    ▲
                    │ SHOULD ALSO
                    │
        ┌───────────┴───────────┐
        │ upload_file           │  ◄── bypasses (RC-022-1)
        │ batch_upload          │  ◄── bypasses (RC-022-2)
        └───────────────────────┘
```

---

## 3. DRY assessment

| Duplication | Locations | Lines (approx) | Severity |
|-------------|-----------|----------------|----------|
| Entity graph write + vector write | `file_upload.rs` vs `merger/` | ~200 | **CRITICAL** |
| Chunk vector upsert loop | `file_upload.rs` vs `build_chunk_vector_batch` | ~40 | HIGH |
| Pipeline bootstrap | `query_bootstrap.rs` (shared) | 0 dup | ✅ |
| Entity normalization | `entity_id.rs` single SSOT | 0 dup | ✅ |
| Query engine wiring | API vs orchestrator (BM25 gap) | partial | MEDIUM |

**DRY score**: **B+ globally, F on upload surface**.

---

## 4. O(n) principles — contract table

### Ingestion

| Step | Ideal | Persister path | file_upload path |
|------|-------|----------------|------------------|
| Chunk vectors | O(1) SQL tx | ✅ UNNEST batch | ❌ O(C) upserts |
| Entity vectors | O(1) SQL tx | ✅ batch | ❌ O(E) upserts |
| Graph nodes | O(1) Cypher batch | ✅ | ❌ O(E) MERGE |
| Graph edges | O(1) Cypher batch | ✅ | ❌ O(R) MERGE |
| LLM extract | O(C) parallel | ✅ | ✅ (shared pipeline) |

### Query

| Step | Complexity | Status |
|------|------------|--------|
| ANN search | O(log V) | ✅ |
| Graph hydrate | O(1) batch per mode | ✅ |
| KV chunk fetch | O(k) | ✅ |
| BM25 rerank | O(k log k) | ✅ API only |
| LLM generate | O(1) call | ✅ |

### Storage admin

| Step | Complexity | Acceptable? |
|------|------------|-------------|
| P-G1b reconcile | O(N nodes) | ✅ admin-only |
| `keys()` full scan | removed | ✅ |

---

## 5. LightRAG parity matrix

| LightRAG concept | EdgeQuake implementation | Gap |
|------------------|-------------------------|-----|
| Text chunking w/ overlap | `chunker` | none |
| Entity/rel extraction per chunk | `LLMExtractor` + gleaning | none |
| Entity merge on insert | `KnowledgeGraphMerger` | **bypassed on sync upload** |
| Entity/rel vector index | pgvector metadata.type | none |
| 6 query modes | `QueryMode` enum | none |
| Keyword extraction | `CachedKeywordExtractor` | none |
| Response w/ citations | `QueryContext` + sources | none |
| Community summaries | — | **not implemented** |

---

## 6. ASCII: intended vs actual dependency flow

**Intended (first principles)**:

```
 HTTP ──► Pipeline ──► IngestionPersister ──► [VectorStorage, GraphStorage]
                           │
                           └── KnowledgeGraphMerger
 SDK  ──► Orchestrator ────┘
 Worker ──► text_insert ───┘
```

**Actual (Code Is Law)**:

```
 text/markdown ──► Worker ──► IngestionPersister ✅

 file upload ──► Pipeline ──► hand-rolled loops ❌

 batch upload ──► Pipeline ──► chunks only ❌
```

---

## 7. Brutal summary

SOLID and O(n) discipline **won inside the pipeline crate** and **lost at the HTTP boundary**. The persister trait is exactly the right abstraction — it is **shamefully underused**. Fixing RC-022-1 is not a refactor for elegance; it is **restoring architectural integrity** promised by plan-19.
