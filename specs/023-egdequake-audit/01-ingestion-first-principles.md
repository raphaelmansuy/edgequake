# 01 — Ingestion Pipeline (First Principles)

> **Spec**: 023-egdequake-audit  
> **Code anchors**: `edgequake-pipeline`, `edgequake-api/services/ingestion_persist.rs`, `edgequake-core/orchestrator/ingestion.rs`

---

## First principle

**Ingestion transforms unstructured text into three durable indexes that must stay consistent:**

1. **Chunk vectors** — semantic lookup (naive/local chunk retrieval)
2. **Entity/relationship vectors** — graph-adjacent semantic lookup (local/global arms)
3. **AGE graph** — structural truth (entities, edges, source tracking, cascade delete)

These three are **not one transaction**. The design choice is explicit: bounded saga, not 2PC fantasy.

---

## Canonical pipeline (the law)

```
  Raw text
     │
     ▼
┌─────────────┐
│  Chunker    │  overlap, token limits, lineage metadata
└──────┬──────┘
       ▼
┌─────────────┐
│  Extractor  │  LLM JSON/tuple → entities + relationships per chunk
│  (parallel) │  EntityId normalization (UPPERCASE_UNDERSCORE)
└──────┬──────┘
       ▼
┌─────────────┐
│  Embedder   │  chunk + entity + relationship embeddings
└──────┬──────┘
       ▼
┌─────────────────────────────────────────────────────────┐
│  DefaultIngestionPersister (SSOT)                       │
│                                                         │
│  Step A: vector_storage.upsert(chunk_vectors)  [atomic] │
│  Step B: KnowledgeGraphMerger.merge(extractions)        │
│          • batched get_nodes_batch                      │
│          • batched upsert_nodes_batch                   │
│          • entity/rel vector batch upsert               │
│          • optional LLM description merge (O(E) LLM)    │
│  On failure: compensate_merge_failure                   │
│              • delete chunk vectors for doc_id          │
│              • rollback partial graph artifacts         │
└─────────────────────────────────────────────────────────┘
       │
       ▼
  invalidate_query_result_cache()
```

**Why vectors first?** Chunk UNNEST upsert is internally atomic. Graph merge is idempotent and source-tracked. Orphan **vectors** are worse than orphan **nodes** because vectors pollute naive retrieval silently.

Evidence: `ingestion_persister.rs:246-306`, `orchestrator/ingestion.rs:266-292`.

---

## Entry points (who calls the persister?)

| Route / caller | Persister? | Saga order? | Cache invalidate? | Grade |
|----------------|------------|-------------|-------------------|-------|
| `POST` text/markdown (async 202) | ✅ via `text_insert` | ✅ | ✅ | A |
| `POST` multipart `upload_file` | ✅ `persist_ingestion_result` | ✅ | ✅ | A |
| `POST` batch upload | ✅ (SPEC-022 P-H2) | ✅ | ✅ | A |
| `EdgeQuake::insert()` orchestrator | ✅ `DefaultIngestionPersister` | ✅ | ✅ | A |
| **`injection` handlers** | ❌ inline merger | ❌ **merge first** | ❌ | **F** |
| Tests / examples (direct merger) | N/A | varies | N/A | — |

**RC-023-1** is the only production-grade regression after SPEC-022.

---

## Injection path autopsy (RC-023-1)

```rust
// injection.rs:965-1021 — WRONG ORDER
let merge_stats = merger.merge(tagged_extractions).await?;   // graph first
for chunk in &result.chunks {
    vector_storage.upsert(&[(chunk_id, embedding, metadata)]).await; // N× round trip
}
```

Problems:

| Issue | First-principles violation |
|-------|---------------------------|
| Merge before chunk vectors | Saga invariant broken — graph exists without searchable chunks |
| Per-chunk `upsert` | O(C) RTTs vs one UNNEST batch |
| No `compensate_merge_failure` | Partial failure → inconsistent triple-store |
| No cache invalidation | Stale query results after injection |
| Truncated content in metadata (`chars().take(500)`) | Citation/naive retrieval may serve truncated text |

---

## Merger semantics (LightRAG fidelity)

| Behavior | EdgeQuake | LightRAG expectation |
|----------|-----------|---------------------|
| Entity name normalization | `EntityId::new` → UPPERCASE_UNDERSCORE | ✅ match |
| Description merge on re-ingest | concat + optional LLM summarize | ✅ match |
| Source document tracking | `source_document_id`, chunk IDs | ✅ match |
| Relationship dedup | merge by src/tgt/type | ✅ match |
| Batch graph writes | `upsert_nodes_batch` | ✅ better than naive LightRAG |

Evidence: `merger/entity.rs`, `entity_id.rs`, `contract_entity_identity.rs`.

---

## Extraction quality (upstream of persistence)

| Component | Assessment |
|-----------|------------|
| Prompts | Ported LightRAG tuple + JSON paths; N-ary decomposition instructions | **Strong** |
| Parser resilience | `JsonExtractionParser` with truncation recovery | **Strong** |
| Gleaning pass | Second LLM pass for missed entities | **Good** (cost ↑) |
| Entity type policy | Strict/permissive schema enforcement | **Good** |
| Mock vs real LLM gap | Mock extracts ~33% fewer unique entities vs real | **Documented risk** |

**Brutal truth**: Persistence cannot fix bad extraction. Garbage entities become permanent graph nodes until admin delete.

---

## O(n) complexity ledger

| Stage | Complexity | Notes |
|-------|------------|-------|
| Chunking | O(n) text | n = doc length |
| Extraction | O(C) LLM calls | C = chunk count; parallelized |
| Embedding | O(C + E + R) API calls | batched where provider allows |
| Chunk vector write | O(1) DB txn | UNNEST batch |
| Entity merge | O(E) graph batch | 2 RTTs: get batch + upsert batch |
| LLM summarization | **O(E) LLM calls** | **RC-023-9 candidate** — disable at scale |
| Injection chunk write | **O(C) RTTs** | **Fix in I1** |

---

## Cross-refs

| Topic | See also |
|-------|----------|
| LightRAG lens | [03-eight-lens-audit.md](./03-eight-lens-audit.md#lens-2-lightrag-expert) |
| Saga / system design | [03-eight-lens-audit.md](./03-eight-lens-audit.md#lens-5-system-engineer) |
| Postgres vector batch | [03-eight-lens-audit.md](./03-eight-lens-audit.md#lens-8-postgres-age-pgvector) |
| Fix injection | [05-improvement-plan.md](./05-improvement-plan.md#i1-close-rc-023-1-injection-persister) |
