# 02 — Query Pipeline Code Audit

> **Cross-ref**: [00-executive](./00-executive-brutal-audit.md) · [03-storage](./03-storage-postgres-age-pgvector.md) · [06-improvement-plan](./06-improvement-plan.md) P-H4–P-H6  
> **Prior work**: SPEC-021 P-G3 (global N+1), P-G6 (engine consolidation), P-G9 (caches)

---

## 1. First principle: query is retrieve-then-generate under a token budget

LightRAG defines six retrieval modes over a **dual index** (vector + graph). EdgeQuake implements all six:

```55:86:edgequake/crates/edgequake-query/src/modes.rs
pub enum QueryMode {
    Naive, Local, Global, Hybrid, Mix, Bypass,
}
```

Pipeline stages (orchestrator doc matches implementation):

```
Query string
     │
     ▼
┌─────────────────┐
│ Keyword extract │  LLM or cache (eq_*_kv suffix -kwcache)
└────────┬────────┘
         ▼
┌─────────────────┐
│ Embed query +   │  CachingEmbeddingProvider (API bootstrap)
│ HL/LL keywords  │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
 vector     graph
 search     batch reads
    │         │
    └────┬────┘
         ▼
┌─────────────────┐
│ Chunk text KV   │  get_by_ids(source_chunk_ids)
└────────┬────────┘
         ▼
┌─────────────────┐
│ BM25 rerank     │  API engine only (see RC-022-4)
└────────┬────────┘
         ▼
┌─────────────────┐
│ Token truncate  │  entities > rels > chunks
└────────┬────────┘
         ▼
┌─────────────────┐
│ LLM answer      │  or Bypass → no LLM
└─────────────────┘
```

---

## 2. Mode-by-mode audit

### 2.1 Naive

- **Vector**: `query_filtered` with `vector_type=chunk` at SQL layer ✅  
- **Why it matters**: Without SQL filter, entity vectors dominate top-k on large graphs (comment in `vector_queries.rs:26-28`)  
- **Complexity**: O(log V) ANN + O(k) KV reads — **good**

### 2.2 Local

- Entity vector search → `get_nodes_batch` + `node_degrees_batch` + `get_edges_for_nodes_batch` ✅  
- Fallback when no entity vectors: `get_popular_nodes_with_degree` (OODA-231) ✅  
- **Complexity**: O(log V) + O(E_batch) — **good** (batch graph ops)

Evidence:

```150:169:edgequake/crates/edgequake-query/src/engine_impl/vector_queries.rs
                graph.get_nodes_batch(&entity_ids),
                graph.node_degrees_batch(&entity_ids),
            ...
            let edges = graph.get_edges_for_nodes_batch(&entity_ids).await?;
```

### 2.3 Global

- Relationship vector search → batch node/degree fetch ✅  
- **P-G3 fix verified** — comment explicitly rejects N+1:

```384:389:edgequake/crates/edgequake-query/src/engine_impl/vector_queries.rs
            // an O(E) N+1 pattern. Local mode already used `node_degrees_batch`;
                graph.get_nodes_batch(&entity_ids),
                graph.node_degrees_batch(&entity_ids),
```

- **Test**: `contract_global_no_nplus1.rs`

### 2.4 Hybrid

- Runs local + global paths, merges contexts  
- **LightRAG fidelity**: high — dual retrieval with keyword split

### 2.5 Mix

- Weighted blend of naive + (local|global) vector scores  
- **Engine contract**: `contract_query_modes.rs` (3/3)  
- **HTTP gap** (from plan-25): HTTP tests check mode + stats, not weight ordering — **RC-022-8 deferred**

### 2.6 Bypass

- Skips LLM generation; returns retrieved context  
- Fixed in engine + HTTP E2E per plan-19

---

## 3. Query engine bootstrap parity (RC-022-4)

**API path** (`query_bootstrap.rs`):

```37:51:edgequake/crates/edgequake-api/src/state/query_bootstrap.rs
        QueryEngine::new(...)
        .with_reranker(reranker)
        .with_embedding_cache()
        .with_result_cache(),
```

**Orchestrator default** (`orchestrator/mod.rs`):

```519:528:edgequake/crates/edgequake-core/src/orchestrator/mod.rs
                QueryEngine::new(...)
                .with_embedding_cache()
                .with_result_cache(),
            )
```

| Feature | API | Orchestrator SDK |
|---------|-----|------------------|
| Embedding cache | ✅ | ✅ |
| Result cache | ✅ | ✅ |
| BM25 reranker | ✅ | **❌** |
| Cache invalidation on insert | ✅ worker + orchestrator | ✅ orchestrator tests |

**Impact**: Rust SDK users and integration tests using bare `EdgeQuake::initialize()` get **worse chunk ranking** than HTTP clients.

---

## 4. Caching architecture

| Cache | Key | Invalidation |
|-------|-----|--------------|
| Keyword | hash(query) + `-kwcache` in KV | TTL / manual |
| Embedding | query text hash | `CachingEmbeddingProvider` |
| Result | query + mode + workspace | `QueryResultCacheInvalidator` on insert |

Orchestrator invalidation tested: `spec021_orchestrator_cache_invalidation.rs`

**Stale result risk**: mitigated for async path; **sync file upload may skip invalidation** (RC-022-1 side effect).

---

## 5. Vector retrieval + Postgres interaction

Local/naive use `query_filtered` which pushes tenant/workspace/type to SQL:

```434:467:edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs
    async fn query_filtered(...) {
        ...
        let filter_sql = mf.build_sql(has_id_filter, 2);
        ...
        ORDER BY embedding <=> $1::vector
        LIMIT ${}
```

Search tuning in transaction (QW3):

```515:527:edgequake/crates/edgequake-storage/src/adapters/postgres/vector/storage_impl.rs
        let iterative_scan = self.supports_iterative_scan().await;
        for stmt in Self::search_tuning_statements(self.index_type, top_k, true, iterative_scan) {
            sqlx::query(&stmt).execute(&mut *tx).await?;
        }
```

On **pgvector 0.7.4**, `iterative_scan_supported = false` → filtered ANN may under-return (see [03-storage](./03-storage-postgres-age-pgvector.md)).

---

## 6. Graph reads during query

Query engine uses **read-only graph trait** (`graph_read()`) with batch methods — good ISP.

AGE backend executes one Cypher query per batch op via `cypher_query` — **O(1) round-trip per batch call**, not per node.

Popular-node fallback uses analytics ops (degree-sorted scan) — acceptable for empty-vector cold start.

---

## 7. Streaming path

`query_stream.rs` delegates to `run_context_pipeline` then streams answer tokens — same retrieval core as non-streaming. Vision/multimodal path in separate contract tests.

---

## 8. Query O(n) complexity table

| Mode | Vector ops | Graph ops | KV ops | LLM calls |
|------|------------|-----------|--------|-----------|
| Naive | 1 ANN | 0 | O(k) | 1 (+0 kw) |
| Local | 1 ANN | 3 batch | O(k) | 2 |
| Global | 1 ANN | 3 batch | O(k) | 2 |
| Hybrid | 2 ANN | 6 batch | O(k) | 2 |
| Mix | 3 ANN | 3-6 batch | O(k) | 2 |
| Bypass | 1-3 ANN | 0-6 batch | O(k) | 0 |

k = configured max chunks (typically ≤60). **No O(N_graph) full scans** in hot path.

---

## 9. Test coverage assessment

| Area | Tests | Grade |
|------|-------|-------|
| Mode semantics | `contract_query_modes.rs` | A |
| Global N+1 | `contract_global_no_nplus1.rs` | A |
| Embedding cache | `contract_embedding_cache.rs` | A |
| Result cache | `contract_query_result_cache.rs` | A |
| HTTP modes E2E | `e2e_spec021_query_modes_http.rs` | A− |
| Postgres + worker ingest → query | **none** | F |
| Mix weight ordering HTTP | **none** | C |

---

## 10. GraphRAG vs LightRAG honesty

EdgeQuake is **LightRAG-faithful**, not **GraphRAG-SOTA**:

- No Leiden/Louvain community detection  
- No hierarchical community summaries  
- No multi-hop planned retrieval beyond fixed batch edge fetch  
- Flat entity-relationship graph with vector anchoring  

**Grade C+ for GraphRAG** is generous — it's a **good LightRAG port** with **production storage hygiene**.

---

## 11. Brutal summary

Query retrieval was the focus of SPEC-021 remediation and **shows it**: batch graph reads, SQL vector filtering, mode contracts, caches. Remaining query debt is **bootstrap parity** (BM25), **infra recall** (pgvector version), and **E2E honesty** (no Postgres worker path test). Do not confuse "query engine is solid" with "GraphRAG intelligence" — the graph is operational, not hierarchical.
