# 01 — Ingestion Pipeline Code Audit

> **Cross-ref**: [00-executive](./00-executive-brutal-audit.md) · [04-first-principles](./04-first-principles-solid-dry-on.md) · [06-improvement-plan](./06-improvement-plan.md) P-H1  
> **Prior work**: SPEC-021 plan-19 P-G2 (persister), P-G2b (async-only text upload)

---

## 1. First principle: ingestion is a transaction across two stores

A document becomes:

1. **Chunk vectors** (pgvector) — semantic search over text spans  
2. **Entity + relationship vectors** (pgvector) — local/global query anchors  
3. **Graph nodes/edges** (AGE) — structure, `source_chunk_ids`, provenance  
4. **KV records** (Postgres JSONB) — chunk text, document metadata, task state  

There is **no distributed transaction**. The correct pattern (implemented in persister):

```
vectors-first (atomic batch) → graph merge (idempotent) → compensate on failure
```

Evidence in orchestrator:

```266:291:edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs
        // ── Cross-store write ordering & saga compensation (SC2 / finding F4) ──
        //
        // WHAT we do instead — a bounded, deterministic SAGA:
        //   1. Write chunk vectors FIRST. This write is internally atomic (QW2
        //      performs a single chunked-UNNEST transaction: all chunk rows for
        //      the document commit together, or none do).
        //   2. Run the graph merge LAST. The graph MERGE is idempotent and
        //      source-tracked, so a failed/partial merge is safe to re-run or to
        //      clean up via normal document deletion.
        //   3. If the merge fails, COMPENSATE by deleting exactly this document's
        //      chunk vectors ...
```

Persister implementation:

```235:306:edgequake/crates/edgequake-pipeline/src/persistence/ingestion_persister.rs
async fn persist_processing_result_impl(...) -> Result<IngestionPersistOutput> {
    let chunk_vectors = build_chunk_vector_batch(result, ctx, chunk_options);
    ...
    if !chunk_vectors.is_empty() {
        vector_storage.upsert(&chunk_vectors).await?;
    }
    let mut merger = KnowledgeGraphMerger::new(...);
    let merge_result = merger.merge(result.extractions.clone()).await;
    match merge_result {
        Ok(stats) if stats.errors == 0 => Ok(...),
        Ok(stats) | Err(merge_err) => {
            compensate_merge_failure(...).await;
            Err(...)
        }
    }
}
```

---

## 2. Entry point inventory (Code Is Law)

| # | Entry | Pipeline | Persistence | Saga | Merger | Grade |
|---|-------|----------|-------------|------|--------|-------|
| A | `EdgeQuake::insert()` | `pipeline.process()` | `DefaultIngestionPersister` | ✅ | ✅ | **A** |
| B | `process_text_insert()` (task worker) | `process_with_resilience_cancellable` | `DefaultIngestionPersister` | ✅ | ✅ | **A** |
| C | `POST /documents/upload` (`upload_file`) | `process_with_resilience` | **inline loops** | ❌ | ❌ | **F** |
| D | `POST /documents/upload/batch` | `process_with_resilience` | **inline loops** | ❌ | ❌ | **D** |
| E | `POST /documents/text` (async) | task → B | via B | ✅ | ✅ | **A** |

**Verdict**: Plan-19 claimed "2 ingestion paths." Code proves **4 write semantics** (A≡B, C≠D≠A).

---

## 3. RC-022-1: `file_upload.rs` — the production footgun

After pipeline processing, the handler **does not** call `IngestionPersister`. It manually:

1. KV-upserts chunks (OK — outside persister scope by design)  
2. **Per-chunk** `vector_storage.upsert` in a loop (O(C) round-trips)  
3. **Per-entity** `graph_storage.upsert_node` (O(E) round-trips, no merge)  
4. **Per-entity** vector upsert (O(E) round-trips)  
5. **Per-relationship** `upsert_edge` (O(R) round-trips)

```314:345:edgequake/crates/edgequake-api/src/handlers/documents/upload/file_upload.rs
    for chunk in &result.chunks {
        if let Some(embedding) = &chunk.embedding {
            ...
            match workspace_vector_storage
                .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                .await
```

```410:415:edgequake/crates/edgequake-api/src/handlers/documents/upload/file_upload.rs
            match state
                .storage
                .graph_storage
                .upsert_node(&entity_key, properties)
                .await
```

```498:502:edgequake/crates/edgequake-api/src/handlers/documents/upload/file_upload.rs
            let _ = state
                .storage
                .graph_storage
                .upsert_edge(&src_key, &tgt_key, properties)
                .await;
```

### What merger gives that this path lacks

| Merger behavior | `KnowledgeGraphMerger` | `file_upload` inline |
|-----------------|------------------------|----------------------|
| Dedup same entity across chunks | ✅ `merge_entities_batch` | ❌ last upsert wins per key only |
| Merge descriptions | ✅ `merge_descriptions` | ❌ overwrite |
| Union `source_chunk_ids` | ✅ | partial (sets on properties but no read-merge) |
| Batched graph writes | ✅ `upsert_nodes_batch` | ❌ N singles |
| Batched entity vectors | ✅ one `upsert` | ❌ N singles |
| Saga on failure | ✅ `compensate_merge_failure` | ❌ partial orphan state |
| LLM summarization option | ✅ | ❌ |
| Query cache invalidation | via processor | **❌ not wired** |

Entity IDs **do** use `EntityId::new` (P-G1 partial credit), but without merge semantics duplicate descriptions and stale vectors persist.

---

## 4. RC-022-2: `batch_upload.rs`

```192:211:edgequake/crates/edgequake-api/src/handlers/documents/upload/batch_upload.rs
    for chunk in &result.chunks {
        if let Some(embedding) = &chunk.embedding {
            ...
            let _ = state
                .storage
                .vector_storage
                .upsert(&[(chunk.id.clone(), embedding.clone(), metadata)])
                .await;
```

Comment admits the problem:

```193:194:edgequake/crates/edgequake-api/src/handlers/documents/upload/batch_upload.rs
    // Note: Batch upload uses default vector storage since there's no workspace context.
    // For workspace-specific storage, use the main upload_file endpoint with tenant context.
```

**No graph writes at all** in batch path — entities/relationships extracted by pipeline are **discarded**. Batch upload is chunk-only ingestion masquerading as full RAG ingest.

---

## 5. Pipeline processing layer (shared — good)

All paths share `Pipeline::process_with_resilience`:

```
content
   │
   ▼
┌──────────────┐     O(C) parallel LLM calls (bounded concurrency)
│ chunk_async  │
└──────┬───────┘
       ▼
┌──────────────┐     resilient_extract_parallel — partial failure OK
│  extraction  │
└──────┬───────┘
       ▼
┌──────────────┐     batch embed chunks + entities
│  embedding   │
└──────┬───────┘
       ▼
 ProcessingResult { chunks, extractions, stats, lineage }
```

**LightRAG alignment**: chunk → extract → embed matches LightRAG's indexing phase. The divergence is entirely in **persistence**, not algorithm.

Adaptive chunk sizing (LightRAG-informed):

```43:64:edgequake/crates/edgequake-core/src/orchestrator/ingestion.rs
fn calculate_adaptive_chunk_size(document_size_bytes: usize) -> usize {
    if document_size_bytes > 100_000 { 600 }
    else if document_size_bytes > 50_000 { 800 }
    else { 1200 }
}
```

---

## 6. Merger layer (canonical write path — good)

```43:72:edgequake/crates/edgequake-pipeline/src/merger/entity.rs
    pub(super) async fn merge_entities_batch(...) -> Result<()> {
        ...
        let existing_map = self.graph_storage.get_nodes_batch(&keys).await?;
        ...
        // single upsert_nodes_batch
```

Vector collection batched:

```13:40:edgequake/crates/edgequake-pipeline/src/merger/entity.rs
    pub(super) fn collect_entity_vector_batch(...) -> Vec<(String, Vec<f32>, serde_json::Value)>
```

**O(n) contract**: one `get_nodes_batch`, one `upsert_nodes_batch`, one vector `upsert` per entity batch — **O(1) storage round-trips per batch**, not O(E).

---

## 7. Async text path (reference implementation)

`text_upload.rs` correctly enqueues background work (P-G2b):

```213:219:edgequake/crates/edgequake-api/src/handlers/documents/upload/text_upload.rs
    // P-G2b (RC-7): force async upload. The synchronous inline-persistence
    // branch (~490 lines that duplicated the processor's chunk/vector/graph
    // writes with N+1 loops and no saga compensation) is removed.
```

**Irony**: sync inline persistence was removed from **text** upload but **survives** in **file** upload.

---

## 8. Ingestion O(n) complexity table

| Phase | Complexity | Dominant cost | Notes |
|-------|------------|---------------|-------|
| Chunking | O(n) text | CPU/tokenizer | n = doc length |
| Extraction | O(C) LLM | API latency | C = chunk count, parallelized |
| Embedding | O(C + E) | API latency | batched by provider limits |
| Persist (persister) | O(1) tx + O(E) merge CPU | Postgres | batched UNNEST |
| Persist (file_upload) | **O(C + E + R) SQL** | **N+1** | **RC-022-1** |
| Summarization (optional) | **O(E) LLM** | API | per-entity when enabled |

---

## 9. Tests that exist vs gaps

| Test | Covers |
|------|--------|
| `contract_ingestion_persistence.rs` | Persister saga, idempotency |
| `contract_merger_graph_batch.rs` | Batch graph writes |
| `contract_entity_identity.rs` | EntityId SSOT |
| `e2e_pipeline_resume.rs` | Checkpoint + text_insert |
| **Missing** | `upload_file` → Postgres → merger semantics E2E |
| **Missing** | Saga failure on sync upload path (no saga to test) |

---

## 10. Brutal summary

The ingestion **algorithm** (pipeline + merger) is production-grade LightRAG. The ingestion **routing** is not: half the HTTP surface area bypasses the fix that SPEC-021 spent weeks building. **Code is law** — and the law currently says "quality depends on which button you clicked."
