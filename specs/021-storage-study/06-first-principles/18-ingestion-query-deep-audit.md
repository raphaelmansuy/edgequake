# 18 — Ingestion & Query Pipeline Deep Audit (Code Is Law)

> **Spec**: 021-storage-study
> **File**: 06-first-principles/18-ingestion-query-deep-audit.md
> **Date**: 2026-06-26
> **Author**: AI Engineering / KnowledgeGraph / LightRAG / System-Design review
> **Method**: First-principles decomposition + "Code is Law" — every claim verified
> against production source in `edgequake/crates/...`. Brutal and honest.
> **Scope**: The full **ingestion** pipeline (chunk → extract → embed → store) and the
> full **query** pipeline (HTTP → engine → retrieval → generation → stream), analyzed
> through DRY, SOLID, first-principles, performance, and O(N) lenses.
> **Relation to prior files**: Extends file 17 (storage-consistency plan) into the
> *algorithmic and architectural* layer. File 17 fixed *which column is read*; this file
> audits *how data is written and retrieved*. New CRITICAL findings (RC-6..RC-11) are
> introduced here and carried into plan-19.
>
> **⚠️ Supersession (2026-06-26, post-`404ce915`)**: §1–§2 ingestion claims describe
> **pre-P-G2** code. RC-7 is fixed (structural); RC-6 fixed for **new writes** via merger.
> For current ingestion truth, read **21-pg2-ingestion-persister-plan.md** and
> **22-pg2-post-ship-brutal-assessment.md**. Keep this file as forensic history.

---

## 0. How to read this document

- Each finding is tagged **VERIFIED** (re-read in source on 2026-06-26) or **NEW**.
- `file:line` references point at the production code that proves the claim.
- Findings are ordered by **severity × user impact**, not chronologically.
- The improvement plan that closes these findings lives in
  `19-ingestion-query-improvement-plan.md`.

---

## 1. The single most damaging finding (read this first)

### RC-6 (CRITICAL, NEW) — Entity identity is not a Single Source of Truth. The production async path silently corrupts the graph + vector index.

A knowledge graph's entire value proposition is **one node per real-world entity**.
EdgeQuake has **three different entity-ID conventions** depending on which write path
ran, and the **production path (async processor) is the least correct**.

**VERIFIED.** Three writers, three schemes:

| Writer | Graph node ID | Entity vector ID | Normalization |
|--------|---------------|------------------|---------------|
| Orchestrator / `KnowledgeGraphMerger` | `normalize_entity_name(name)` → `JOHN_DOE` | **bare** `JOHN_DOE` | yes |
| Sync text upload (`text_upload.rs`) | `normalize_entity_name(name)` → `JOHN_DOE` | `entity:JOHN_DOE` | yes |
| **Async processor (`text_insert.rs`) — PRODUCTION** | **raw `entity.name`** → `John Doe` | `entity:John Doe` | **NO** |

Evidence:

```874:874:edgequake/crates/edgequake-api/src/processor/text_insert.rs
                nodes_batch.push((entity.name.clone(), properties));
```

```1004:1004:edgequake/crates/edgequake-api/src/processor/text_insert.rs
                    let entity_id = format!("entity:{}", entity.name);
```

```489:512:edgequake/crates/edgequake-api/src/handlers/documents/upload/text_upload.rs
                let entity_key = normalize_entity_name(&entity.name);
                ...
                    let entity_id = format!("entity:{}", entity_key);
```

```15:27:edgequake/crates/edgequake-pipeline/src/merger/entity.rs
        let entity_key = normalize_entity_name(&entity.name);
        ...
            self.vector_storage
                .upsert(&[(entity_key.clone(), embedding.clone(), metadata)])
```

The **reader** side expects the `entity:{name}` prefix and decodes via
`VectorId::from_storage_id` (`strategies/mod.rs:45-73`).

**First-principles consequence**: the KG invariant "one node per entity" is *unenforced*.
When the LLM extracts `"John Doe"` in one chunk and `"john doe"` in another, the async
processor creates **two graph nodes** (`John Doe` + `john doe`) and **two entity
vectors** (`entity:John Doe` + `entity:john doe`). The merger would have collapsed them
to `JOHN_DOE`. The query engine's `decode_entity_name_from_result` then fetches the
graph node `John Doe` which exists, but a query embedding that matched `entity:john doe`
would fetch `john doe` — *same real entity, two nodes, split degree, split edges,
split context*. Recall is silently halved for the affected entity.

This is strictly worse than the file-16 "0 entities" bug: that bug *displayed* 0; this
bug *displays a correct-looking number while serving a fragmented graph*. It is the
deepest correctness defect in the system.

**Compounding fact**: the orchestrator path (which *does* normalize) is **not used by
production uploads** — all user traffic goes through the async processor. The
"correct" merger code is effectively dead for the main flow.

---

## 2. Ingestion pipeline — end-to-end map

### 2.1 There is not one ingestion pipeline. There are three.

```text
                    ┌─────────────────────────────────────┐
                    │   edgequake-pipeline (SHARED compute)│
                    │   chunk → extract → embed            │
                    └──────────────┬──────────────────────┘
                                   │
        ┌──────────────────────────┼──────────────────────────┐
        ▼                          ▼                          ▼
 EdgeQuake::insert()     DocumentTaskProcessor       sync text_upload
 (orchestrator/           (text_insert.rs)            (inline, in handler)
  ingestion.rs)           *** PRODUCTION ***          no merger, no saga
        │                          │                          │
 batched chunk vectors    per-chunk vector loop       per-chunk vector loop
 KnowledgeGraphMerger     manual graph batches        manual single upserts
 normalize entity (✓)     RAW entity name (✗)         normalize entity (✓)
 saga on merge fail       partial saga (nodes only)   NO saga
 NO KV / NO relational    KV + relational dual-write  KV + relational
```

**VERIFIED.** The divergence happens immediately after `ProcessingResult` is produced.
The three paths share the compute layer but **diverge completely at persistence**, with
**inverted batching characteristics** (see §4).

### 2.2 Storage-write sequence — async processor (production)

`process_text_insert` (`text_insert.rs:37-1283`) — a **single 1,247-line method** —
performs, in order:

| # | Stage | Store | Lines | Batching |
|---|-------|-------|-------|----------|
| 1 | Metadata enrich | KV | 83-149 | batch |
| 2 | Compute (pipeline) | — | 360-541 | — |
| 3 | Checkpoint save | KV | 522-538 | full `ProcessingResult` clone |
| 4 | Status transitions | KV | 217-740 | per-stage |
| 5 | Chunks | KV | 587-620 | **batch** |
| 6 | Chunk vectors | Vector | 686-714 | **ONE upsert per chunk (N+1)** |
| 7 | Graph prefetch nodes | Graph | 785-812 | batch |
| 8 | Graph prefetch edges | Graph | 830-844 | **ONE `get_edge` per rel (N+1)** |
| 9 | Graph nodes upsert | Graph | 922-977 | **batch** (UNWIND on postgres) |
| 10 | Entity vectors | Vector | 988-1022 | **ONE upsert per entity (N+1)** |
| 11 | Graph edges upsert | Graph | 1047-1058 | **batch** |
| 12 | Final stats | KV + relational | 1128-1177 | batch |
| 13 | Lineage | KV | 1188-1219 | batch |
| 14 | Checkpoint clear | KV | 1249 | — |

### 2.3 Storage-write sequence — orchestrator (library/examples, NOT production uploads)

`EdgeQuake::insert` (`ingestion.rs:127-437`):

| # | Stage | Store | Lines | Batching |
|---|-------|-------|-------|----------|
| 1 | Compute | — | 262-265 | — (fail-fast `process`, not resilient) |
| 2 | Chunk vectors | Vector | 303-336 | **single batched upsert** |
| 3 | Entities + rels + entity vectors | Graph + Vector | 353-423 | **per-entity sequential** via merger |

**No KV. No `documents` table. No metadata. No lineage.** The orchestrator is a
library-era code path that the production API does not use for uploads.

---

## 3. Query pipeline — end-to-end map

### 3.1 Three query engines coexist

| Engine | File | Status | Algorithm |
|--------|------|--------|-----------|
| `SOTAQueryEngine` | `sota_engine/*` | **PRODUCTION** | LightRAG-inspired multi-level |
| `QueryEngine` (legacy) | `engine.rs` | bootstrapped but HTTP-forbidden | naive popular-labels walk |
| `strategies/*` | `strategies/*.rs` | **benchmark-only** (per `mod.rs:1-4`) | near-duplicate of SOTA |

**VERIFIED** that legacy `QueryEngine` is still constructed (`query_bootstrap.rs:33-39`)
but the HTTP path forbids it (`spec017_query_production_path_contract.rs`). It is dead
weight that misleads readers and benchmarks.

### 3.2 SOTA query flow (production)

```text
POST /api/v1/query | /query/stream | /chat/completions/stream
  → execute_sota_query[_stream]_with_auth_fallback
  → SOTAQueryEngine::run_query_pipeline   (query_pipeline.rs:52-392)
      prepare → retrieve → finalize
  → API post-processing (sources, KV metadata, FAKE rerank in sync path)
  → JSON | SSE
```

**Prepare** (`query_pipeline.rs:147-214`): parallel `tokio::join!` of (a) LLM keyword
extraction via `CachedKeywordExtractor` and (b) `embed_one`. Then `validate_keywords`
(L parallel `search_labels`), mode selection, `QueryEmbeddings::compute_with_query_vec`
(batch embed high/low keyword texts).

**Retrieve** (`query_pipeline.rs:217-302` → `vector_queries.rs`) — mode dispatch:

| Mode | Vector reads | Graph reads | Notes |
|------|--------------|-------------|-------|
| Naive | 1× `query_filtered` (type=chunk) | none | |
| Local | 1× entity search + optional 1× chunk-by-ID | batch nodes/degrees/edges; popular fallback | **fixed** N+1 |
| Global | 1× rel search + optional 1× chunk-by-ID | batch nodes; **N× `node_degree`** | **N+1 NOT fixed** |
| Hybrid | `tokio::join!(local, global, naive)` → round-robin | up to 2× graph batch sets | **3× cost** |
| Mix | delegates to Hybrid | same | **docs lie** |
| Bypass | none | none | **broken at HTTP** |

**Finalize** (`query_pipeline.rs:305-392`): document filter → optional BM25 rerank →
`sort_entities_by_degree` → `balance_context` truncation → LLM `complete`/`stream` →
`build_prompt`.

### 3.3 Storage reads in order (typical Hybrid query)

1. Keyword cache (in-memory LRU 1000, 24h TTL)
2. LLM keyword extraction (cache miss)
3. **L parallel** `graph.search_labels(kw, 1)` — each may run FTS + trigram + ILIKE
4. 1–3 embedding API calls
5. Local arm: `query_filtered(low_level, 3E)` → `get_nodes_batch` + `node_degrees_batch` + `get_edges_for_nodes_batch` → `query_filtered(low_level, K, Some(chunk_ids))`
6. Global arm: `query_filtered(high_level, 3R)` → `get_nodes_batch` → **per-entity `node_degree`** → `query_filtered(high_level, K, Some(chunk_ids))`
7. Naive arm: `query_filtered(query, K, type=chunk)`
8. In-memory round-robin merge
9. Document filter, optional BM25 rerank, truncation
10. LLM complete/stream
11. API: **M unique KV metadata lookups** for source titles (`mod.rs:101-120`)

---

## 4. Performance & O(N) analysis (the brutal numbers)

**Legend**: N = corpus vectors, C = chunks, E = entities (≤60), R = relationships (≤60),
K = chunks (≤20), L = low-level keywords, D = embedding dimension, W = workspace KV keys.

### 4.1 Ingestion hot paths

| Hot path | Complexity | Evidence | Verdict |
|----------|------------|----------|---------|
| Chunking | O(N_chars) | `chunker/mod.rs:107-132` | OK |
| LLM extraction | O(C) calls, bounded concurrency | `extraction.rs:41-85` | OK (C round-trips inherent) |
| Embedding (pipeline) | O(C+E+R) sub-batched | `embeddings.rs:123-208` | Good |
| **Chunk vector write (processor)** | **O(C) DB round-trips** | `text_insert.rs:686-714` | **BAD** — orchestrator batches (`ingestion.rs:331-335`) |
| **Entity vector write (processor)** | **O(E) round-trips** | `text_insert.rs:988-1022` | **BAD** |
| Graph node upsert (processor) | O(E) batched UNWIND | `nodes_ops.rs:112+` | Good (postgres) |
| **Graph edge prefetch (processor)** | **O(R) `get_edge` calls** | `text_insert.rs:830-844` | **N+1** |
| Graph edge upsert | O(R) batched | `text_insert.rs:1047-1058` | OK |
| **Merger (orchestrator)** | **O(E+R) sequential** — vector+graph per item | `merger/entity.rs:14-48`, `merger/mod.rs:216-260` | **No batching at merge layer** |
| Sync upload graph | **O(E+R) single upserts** | `text_upload.rs:431-585` | Worst path |
| **KV `keys()` scan** | **O(W)** | `reprocess.rs:64-118`, `pdf_processing.rs:287-292` | **Catastrophic at scale** |
| Checkpoint save | O(result size) full clone to KV | `pipeline_checkpoint.rs:127-145` | Multi-MB KV values |
| Progress metadata | O(C/3) fire-and-forget KV writes | `text_insert.rs:260-298` | Contention under load |

**Key insight — inverted batching**: the orchestrator batches chunk vectors but not the
merger; the processor batches graph nodes/edges but not chunk/entity vectors. **Neither
path batches everything.** A correct design batches all vector writes and all graph
writes; each path implements half of that.

### 4.2 Query hot paths

| Hot path | Complexity | Evidence | Verdict |
|----------|------------|----------|---------|
| Vector search (memory) | **O(N·D + N log N)** brute-force | `memory/vector.rs:102-117` | Dangerous if used at scale |
| Vector search (postgres) | ~O(log N) ANN (HNSW/IVFFlat) | `ddl.rs:38-43`, `search_tuning.rs:22-32` | Good — true ANN |
| Filtered ANN (chunk-by-ID) | iterative_scan, capped 20k tuples | `storage_impl.rs:515-527` | OK with tuning |
| **Global `node_degree` loop** | **O(E) round-trips** | `vector_queries.rs:387-392` | **N+1 — Local was fixed, Global wasn't** |
| Hybrid | **3× parallel retrieval** (local+global+naive) | `vector_queries.rs:460-481` | 3× cost vs naive |
| Hybrid merge | O(K) round-robin + HashSet dedup | `vector_queries.rs:508-525` | OK; **no RRF** |
| Keyword validation | L parallel `search_labels` (FTS→trigram→ILIKE) | `query_ops.rs:129-225` | O(log V)→O(V) × L |
| Entity sort | O(E log E) | `reranking.rs:113-116` | OK |
| Truncation | 2-pass O(items × tokens) | `truncation.rs:78-253` | OK |
| **API source-title fetch** | **M KV `get_by_id` per source** | `mod.rs:101-120` | **N+1** |
| **No query-result cache** | — | — | every request re-embeds + re-retrieves |
| **No query-embedding cache** | — | — | every request re-embeds the query |

### 4.3 The O(W) KV scan — a lurking landmine

`reprocess.rs:64-118` and `pdf_processing.rs:287-292` call `kv_storage.keys()` (or
`keys_like`) which scans **every key in the workspace** to find `{doc_id}-chunk-*`.
For a workspace with 10k documents × 30 chunks = 300k keys, every reprocess and every
PDF resume scans 300k keys. This is the same class of defect as the file-11 dashboard
scan, now on the ingestion path.

---

## 5. DRY violations (code-verified)

### 5.1 Ingestion — three persistence implementations of the same logic

- **Chunk KV write**: `text_upload.rs:359-375` ≈ `text_insert.rs:587-605` (identical shape, processor adds line/offset fields).
- **Per-chunk vector loop**: `text_upload.rs:386-407` ≈ `text_insert.rs:686-714` (same N+1 `upsert(&[(single)])`).
- **Entity graph+vector**: sync normalizes (`text_upload.rs:489-514`), async does not (`text_insert.rs:874, 1004`) — *divergent, not just duplicated*.
- **Status/stats/relational dual-write** triplicated: `text_upload.rs:619-714`, `text_insert.rs:1128-1177`, `status_updates.rs:293-475`.
- **`compensation.rs`** claims DRY convergence (`compensation.rs:3-8`) but is only wired into 1.5 of the 3 paths (see §7).

### 5.2 Query — two engines + orphaned strategies + dead module

- Legacy `QueryEngine` (`engine.rs:341-685`) duplicates retrieval+prompt+generation with an **inferior** algorithm (popular labels, per-entity `get_node`+`node_degree`+`get_node_edges` N+1 at `engine.rs:548-608`).
- `strategies/*` (~400 LOC) near-duplicates `vector_queries.rs`: Local `strategies/local.rs:44-116` vs `vector_queries.rs:48-239`; Global `strategies/global.rs:43-123` vs `vector_queries.rs:243-443` (Global strategy *also* has the N+1 `node_degree`).
- `chunk_retrieval.rs:41-186` is **dead code** that invents chunk IDs `{entity}_chunk` and contains a fake rerank — not used by SOTA path.
- Local vs Global chunk-collection blocks duplicated: `vector_queries.rs:178-237` ≈ `vector_queries.rs:396-441`.
- `enrich_retrieved_context` (`query_pipeline.rs:108-145`) duplicates filter+rerank+sort+`balance_context` from `pipeline_finalize` (`query_pipeline.rs:314-347`) — two code paths for the same logic (streaming vs non-streaming).

### 5.3 Query — API fake rerank vs engine real BM25

**VERIFIED + NEW.** The engine implements real BM25 via a `Reranker` trait
(`reranking.rs:9-106`). The **sync API handler** silently replaces it with a **fake**
rerank that mutates scores by `score * 0.95 + 0.05` and reports a hardcoded `5ms`:

```229:237:edgequake/crates/edgequake-api/src/handlers/query/query_execute.rs
    let reranked = request.enable_rerank;
    let rerank_time_ms = if reranked {
        // Simulate rerank time for now - actual implementation would call rerank API
        Some(5u64)
```

This means: enabling `rerank` on the sync `/query` endpoint does **nothing** to
ordering while *appearing* to. The streaming path uses the real reranker. Two paths,
two truths.

---

## 6. SOLID violations (code-verified)

### 6.1 SRP — god objects and god functions

| Location | Issue |
|----------|-------|
| `process_text_insert` (`text_insert.rs:37-1283`) | **1,247-line method**: cancel gates, checkpoint, pipeline, 4 storage backends, status, lineage, cache invalidation |
| `text_upload.rs:68-755` | Handler doing full ingestion persistence + metrics |
| `pdf_processing.rs:136-924` | PDF conversion + vision fallback + KV + relational linking + handoff |
| `reprocess.rs:31-602` | KV scan + postgres scan + routing + cleanup in one handler |
| `SOTAQueryEngine` (`sota_engine/mod.rs:315-328`) | Owns keyword extraction, validation cache, embedding orchestration, 5 retrieval modes, merging, reranking, truncation, prompt building, LLM generation (text + vision), streaming |

### 6.2 OCP — adding a query mode requires editing 4+ files

To add mode **X**: `modes.rs` (enum+flags) → `vector_queries.rs` (new method) →
`query_modes.rs` (delegate) → `query_pipeline.rs:229-301` (×2 match arms for
Some/None vector storage) → API docs/tests → legacy `strategies/` + factory. **No
plugin registry.** Closed for modification across many files.

### 6.3 LSP — batch trait default is a performance trap

```17:24:edgequake/crates/edgequake-storage/src/traits/graph_mutate_ops.rs
    async fn upsert_nodes_batch(
        &self,
        nodes: &[(String, HashMap<String, serde_json::Value>)],
    ) -> Result<()> {
        for (node_id, properties) in nodes {
            self.upsert_node(node_id, properties.clone()).await?;
        }
```

Postgres overrides with UNWIND (`nodes_ops.rs:112+`); **memory adapter inherits the N+1
default**. Callers assume "batch" semantics; the performance contract is
backend-dependent. **Tests pass on memory (fast N+1) while production uses postgres
(UNWIND) — performance tests can lie.** This is an LSP violation because the subtype
(memory) does not honor the performance contract callers assume of the supertype.

### 6.4 ISP — fat storage traits

- `DocumentTaskProcessor` constructor takes 10+ dependencies (`mod.rs:243-253`).
- Query retrieval holds `Arc<dyn GraphStorage>` (full mutate trait) on the **read** path; partial mitigation via `GraphReadView` (`sota_engine/mod.rs:369-373`) but the engine struct still owns the fat trait.
- `GraphStorageAnalyticsOps` default impls **ignore workspace scoping** (`graph_analytics_ops.rs:30-37` — `node_count_by_workspace` calls `node_count()`), an ISP/LSP hole that can leak cross-workspace counts.

### 6.5 DIP — high-level reaching into concrete handler layers

- `text_upload.rs` calls `state.storage.graph_storage` directly — no ingestion port abstraction.
- Processor reaches into `crate::handlers::workspaces::invalidate_workspace_stats_cache` (`text_insert.rs:1241`) — a processor depending on a handler.
- `reprocess.rs:472-474` uses `crate::handlers::pdf_upload::types` — cross-handler coupling.

---

## 7. Saga / error-handling asymmetry (code-verified)

### 7.1 Orchestrator saga (strong)

```371:422:edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs
        let merge_stats = match merger.merge(processing_result.extractions.clone()).await {
            Ok(stats) if stats.errors == 0 => stats,
            Ok(stats) => {
                return Err(Self::fail_with_chunk_vector_rollback(/* ... */).await);
            }
            Err(merge_err) => {
                return Err(Self::fail_with_chunk_vector_rollback(/* ... */).await);
            }
        };
```

Vectors first, graph last, **compensate vectors on any graph failure.**

### 7.2 Processor saga (partial, leaky)

Compensation is wired **only** for the node-batch failure:

```922:943:edgequake/crates/edgequake-api/src/processor/text_insert.rs
        if !nodes_batch.is_empty() {
            if let Err(e) = self.graph_storage.upsert_nodes_batch(&nodes_batch).await {
                ...
                edgequake_storage::compensation::compensate_orphan_vectors(
                    workspace_vector_storage.as_ref(),
                    &document_id,
                    &written_chunk_vector_ids,
                    &[],
                    &err_msg,
                )
```

**Orphan scenarios the processor does NOT clean up:**

| Failure point | Orphan data |
|---------------|-------------|
| KV chunks written, vector loop partial | Chunks in KV, subset of vectors |
| Node batch OK, entity vector failures | Graph nodes + chunk vectors, missing entity vectors (`1024-1038` warn-and-continue) |
| **Edge batch fails** | Nodes + entity vectors + chunk vectors, partial edges (`1047-1058`) — **no compensation** |
| Checkpoint saved, crash before clear | Stale checkpoint; resume may skip re-extraction |
| Sync upload path | **No compensation at all** |
| Merger per-entity vector writes inside `merge` | **No rollback of prior entities** written earlier in the same `merge` call |

`compensation.rs:3-8` is **aspirational documentation**, not accurate for the processor
edge/entity-vector failures.

---

## 8. LightRAG fidelity — honest assessment

### 8.1 What roughly matches

| LightRAG step | EdgeQuake |
|---------------|-----------|
| High/low keyword extraction | LLM + `QueryEmbeddings` (`sota_engine/mod.rs:183-312`) |
| Local = entity-level retrieval | Entity vector search + graph hydrate |
| Global = relationship-level | Relationship vector search |
| Naive chunk search | Type-filtered chunk VDB |
| Token budgets | 30K total, 10K/10K/10K split |

### 8.2 Major deviations (brutal)

1. **No community detection / community summaries.** Docs claim "community summaries"
   (`modes.rs:64-65`, `sota_engine/mod.rs:8`) but **zero implementation** (no
   leiden/louvain/cluster in the query crate). Global = relationship vectors, *not*
   LightRAG communities. **Documentation overclaims.**
2. **Mix mode ≠ weighted blend** — documented as "configurable weights"
   (`modes.rs:73-75`) but code is a `query_hybrid` alias (`vector_queries.rs:578-586`).
   **Docs lie.**
3. **Hybrid adds a naive third arm** — LightRAG hybrid is local+global; EdgeQuake runs
   local+global+naive in parallel (`vector_queries.rs:460-481`). Broader recall, 3× cost,
   different fusion.
4. **Bypass mode is broken** at the HTTP/SOTA layer. LightRAG bypass = direct LLM.
   SOTA returns an empty-context apology (`prompt.rs:200-204`). Only `sota_bridge.rs`
   handles bypass correctly for orchestrator callers. Test only asserts empty chunks,
   not answer quality.
5. **No RRF (Reciprocal Rank Fusion)** anywhere — Hybrid merge is round-robin, not
   score-based fusion.
6. **"SOTA" naming is marketing.** The stack is a solid LightRAG-*inspired* engine with
   real EdgeQuake fixes (OODA-230/231 chunk starvation, SPEC-007 SQL filters,
   workspace isolation). It is **not** state-of-the-art retrieval research: no learned
   sparse-dense fusion, no cross-encoder reranker by default, no graph neural retrieval.

---

## 9. Caching gaps (code-verified)

| Cache | What | TTL | On query path? |
|-------|------|-----|----------------|
| Keyword cache | LLM-extracted keywords | 24h | Yes |
| Keyword validation cache | keyword → exists_in_graph | in-memory, max 10k | Yes |
| CacheManager | Conversations + message lists | 5min/1min | **No** (chat persistence only) |
| **Query-result cache** | — | — | **None** |
| **Query-embedding cache** | — | — | **None — every request re-embeds the query** |

Every query pays: 1–3 embedding calls + LLM keyword extraction (on cache miss) + 1–5
vector searches + LLM generation. The query embedding is **recomputed on every request
even for the same query string**. For a chat UI re-asking similar questions, this is
pure waste.

---

## 10. Findings registry (consolidated, with severity)

| ID | Severity | Status | Finding | Fixed by |
|----|----------|--------|---------|----------|
| RC-6 | **CRITICAL** | ✅ Fixed (new writes) / ⚠️ legacy | Entity identity SSOT; async raw-name path removed via P-G2 merger delegation | plan-19 P-G1 + P-G2a |
| RC-7 | **CRITICAL** | ✅ FIXED (structural) | Three ingestion paths collapsed to `persist_processing_result` (P-G2a) | plan-19 P-G2, plan-21, plan-22 |
| RC-8 | **HIGH** | ✅ FIXED | Global mode N+1 `node_degree` | plan-19 P-G3 |
| RC-9 | **HIGH** | ✅ FIXED | Merger batched vector upserts (P-G4-merger) | plan-23 |
| RC-10 | **HIGH** | ✅ FIXED (new writes) | `compensate_merge_failure` + `MergeArtifacts` | plan-23 P-G5 |
| RC-11 | **HIGH** | ⛔ NEW | Two query engines + dead strategies + dead `chunk_retrieval.rs`; API fake rerank contradicts engine BM25 | plan-19 P-G6 |
| RC-12 | **MEDIUM** | ⛔ NEW | O(W) KV `keys()` scans on reprocess + PDF resume | plan-19 P-G7 |
| RC-13 | **MEDIUM** | ⛔ NEW | Bypass mode broken at HTTP; Mix mode == Hybrid (docs lie) | plan-19 P-G8 |
| RC-14 | **MEDIUM** | ⛔ NEW | No query-result / query-embedding cache | plan-19 P-G9 |
| RC-15 | **MEDIUM** | ⛔ NEW | LSP batch-default trap: memory adapter inherits N+1; perf tests lie | plan-19 P-G10 |
| RC-16 | **LOW** | ⛔ NEW | Streaming lacks backpressure and vision parity with sync path | plan-19 P-G11 |
| RC-17 | **LOW** | ⛔ NEW | `GraphStorageAnalyticsOps` default impls ignore workspace scoping (cross-workspace count leak) | plan-19 P-G12 |
| RC-18 | **HIGH** | ✅ FIXED (2026-06-26) | Heavy ingestion triggered false "backend not reachable" banner — UI probed deep `/health` (DB pings) with 2s timeout while process was alive; pool saturation conflated with downtime | plan-19 P-G13 — `/live` liveness gate, bounded storage pings (750ms), stale-if-error stats, degraded banner copy |
| RC-19 | **HIGH** | ✅ FIXED (2026-06-26) | Tenant-fairness requeue + worker retry/orphan recovery minted **new document UUIDs** per attempt when `existing_document_id` was not persisted before KV metadata; multiple PdfProcessing tasks for same `pdf_id` under pressure → duplicate rows | plan-19 P-G14 — `ingest_admission.rs` SSOT, single-flight enqueue, queued KV shell at upload time |
| (file-16 RC-1..5) | CRITICAL→LOW | see file 17 | relational `documents.entity_count` dead columns; per-row entity_count; workspace drift; NULL status; inspector trusts dead column | plan-17 Phase A–F |

---

## 11. First-principles diagnosis — why these defects exist

1. **The compute layer was refactored; the persistence layer was not.** `edgequake-pipeline`
   is shared and clean. Persistence was bolted on three times (orchestrator, processor,
   sync handler) with no shared `IngestionPersister` abstraction. DRY was enforced in the
   wrong place.

2. **The "correct" path (orchestrator+merger) was bypassed by production.** The async
   processor was built for resilience (checkpoints, cancellation, progress) but
   re-implemented persistence *worse* than the merger it replaced. The merger's
   normalization, batching, and saga were left on the library path.

3. **Identity was never promoted to a first-class concept.** `vector_id.rs` documents
   the contract but no writer is *bound* to it. There is no `EntityId` newtype that
   forces normalization at construction. SSOT is documented, not enforced.

4. **The query layer accreted instead of migrated.** `QueryEngine` → `strategies` →
   `SOTAQueryEngine` are three generations living in the same crate. Each generation
   kept for "benchmarks" or "compatibility", leaving three sources of truth for
   retrieval logic.

5. **Performance contracts are not part of the trait system.** `upsert_nodes_batch`
   has a default that is O(N). The trait promises *semantics* but not *performance*.
   Backends with different performance profiles silently violate caller assumptions.

6. **Documentation tracks intent, not reality.** `compensation.rs` documents
   convergence that isn't wired. `modes.rs` documents Mix weights that don't exist.
   `sota_engine/mod.rs` documents community summaries that aren't implemented. Each
   stale doc is a future bug for a developer who trusts it.

7. **Liveness and capacity were conflated in the UI probe.** The dashboard banner
   treated a slow deep `/health` (storage `ping()` waiting on a saturated DB pool
   during PDF ingestion) as "backend not reachable". Terminal logs during 3 concurrent
   PDF jobs show `/health` completing in 2–4ms — the backend was alive; the false
   banner came from probing the wrong tier with too-aggressive timeouts and no retry.
   First principle: **process liveness (`/live`) must never share the ingestion pool
   budget**; deep health may return `degraded` without implying downtime.

---

## 12. What this audit does NOT cover (out of scope)

- The storage-consistency findings (RC-1..5) already owned by file 16/plan-17.
- Frontend API client DRY/SOLID (owned by file 14).
- Graph materialization capacity (owned by file 15).
- Auth, rate-limiting, conversation persistence internals.

These remain valid; this file is *additive* on the ingestion/query *algorithmic* layer.

---

## 13. Task logs

Actions: Read the full ingestion path (orchestrator/ingestion.rs, processor/text_insert.rs 1284 lines, pdf_processing.rs, text_upload.rs, reprocess.rs, merger/*, pipeline/helpers/embeddings.rs, compensation.rs) and the full query path (engine.rs, sota_engine/*, strategies/*, vector_queries.rs, query_ops.rs, postgres vector storage_impl.rs, memory vector.rs). Dispatched two parallel read-only exploration subagents for ingestion and query; cross-verified their reports against my own reads. Verified the RC-6 entity-ID violation by direct grep of `nodes_batch.push((entity.name.clone(), ...))` and `format!("entity:{}", entity.name)` vs `normalize_entity_name` callers.

Decisions: Classified RC-6 (entity identity SSOT) as the single most damaging finding — worse than file-16's "0 entities" because it silently fragments the graph instead of displaying 0. Re-elevated the async-processor path as the de-facto production implementation and the *least* correct. Confirmed the query layer has three generations of code with two of them dead/fake. Carried all findings into plan-19 with concrete file:line fixes.

Next steps: Author `19-ingestion-query-improvement-plan.md` with phases G1–G12 mapping 1:1 to RC-6..RC-17, ordered by correctness-then-performance, each with files/edge-cases/acceptance tests. Then update README index.

Lessons/insights: The deepest lesson is the **abstraction-inversion pattern**: the codebase has a clean shared compute layer (`edgequake-pipeline`) but persisted the *output* of that layer through three ad-hoc paths, none of which reuse a shared persistence abstraction. The merger is the closest to correct, and it is the one production bypasses. Future refactors must promote *persistence* (not just compute) to a single trait-backed `IngestionPersister` and bind entity identity to a newtype that cannot be constructed un-normalized.
