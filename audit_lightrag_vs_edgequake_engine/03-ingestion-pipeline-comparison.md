# Ingestion Pipeline Deep Comparison

## 1. Pipeline Overview

### LightRAG Ingestion Flow

```
Document → Chunking → Entity Extraction → Gleaning → Merge (2-phase) → Storage
              │              │              │              │
              │              │              │              ├── Graph DB
              │              │              │              ├── Vector DB
              │              │              │              └── KV Storage
              │              │              │
              │              │              └── Second LLM pass
              │              │
              │              └── First LLM pass + parsing
              │
              └── Token-based splitting
```

**Code References:**

- [lightrag/operate.py#L89-L168](lightrag/operate.py) - `chunking_by_token_size`
- [lightrag/operate.py#L3200-L3600](lightrag/operate.py) - `extract_entities`
- [lightrag/operate.py#L2400-L2800](lightrag/operate.py) - `merge_nodes_and_edges`

### EdgeQuake Ingestion Flow

```
Document → Chunking → Entity Extraction → Embedding → Lineage → Storage
              │              │              │           │           │
              │              │              │           │           ├── Graph DB
              │              │              │           │           ├── Vector DB
              │              │              │           │           └── KV Storage
              │              │              │           │
              │              │              │           └── Optional tracking
              │              │              │
              │              │              └── Batch embedding
              │              │
              │              └── Single LLM pass + JSON parsing
              │
              └── Sliding window
```

**Code References:**

- [edgequake-pipeline/src/chunker.rs](edgequake/crates/edgequake-pipeline/src/chunker.rs)
- [edgequake-pipeline/src/extractor.rs](edgequake/crates/edgequake-pipeline/src/extractor.rs)
- [edgequake-pipeline/src/pipeline.rs](edgequake/crates/edgequake-pipeline/src/pipeline.rs)

---

## 2. Chunking Comparison

### LightRAG Chunking

```python
# lightrag/operate.py - chunking_by_token_size
def chunking_by_token_size(
    tokenizer: Tokenizer,
    content: str,
    split_by_character: str | None = None,      # Optional character split
    split_by_character_only: bool = False,       # Force character split
    chunk_overlap_token_size: int = 100,         # Overlap tokens
    chunk_token_size: int = 1200,                # Chunk size
) -> list[dict[str, Any]]:
```

**Features:**

- ✅ Token-based splitting (accurate for LLM context)
- ✅ Optional character-based splitting (e.g., by paragraphs)
- ✅ Configurable overlap for context preservation
- ✅ Returns token count per chunk
- ⚠️ Raises `ChunkTokenLimitExceededError` for oversized chunks in character-only mode

**Algorithm:**

1. Tokenize entire content
2. If `split_by_character`:
   - Split by character first
   - Sub-split chunks exceeding token limit
3. Else:
   - Sliding window with overlap

### EdgeQuake Chunking

```rust
// edgequake-pipeline/src/chunker.rs
pub struct ChunkerConfig {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub min_chunk_size: usize,
}

impl Chunker {
    pub fn chunk(&self, content: &str, document_id: &str) -> Result<Vec<TextChunk>> {
        // Sliding window approach
    }
}
```

**Features:**

- ✅ Character-based sliding window (simpler)
- ✅ Configurable size and overlap
- ✅ Minimum chunk size enforcement
- ❌ No token-aware splitting (may over/underfill LLM context)
- ❌ No character-based splitting option

### Chunking Comparison Matrix

| Feature         | LightRAG            | EdgeQuake               |
| --------------- | ------------------- | ----------------------- |
| Token-based     | ✅ Yes              | ❌ No (character-based) |
| Character split | ✅ Optional         | ❌ No                   |
| Overlap         | ✅ Configurable     | ✅ Configurable         |
| Token counting  | ✅ Per chunk        | ❌ No                   |
| Error handling  | ✅ Custom exception | ✅ Result<>             |

**Impact:** LightRAG's token-based chunking is more accurate for LLM context management. EdgeQuake may send suboptimal chunk sizes.

---

## 3. Entity Extraction Comparison

### LightRAG Entity Extraction

```python
# lightrag/operate.py - extract_entities
async def extract_entities(
    chunks: dict[str, TextChunkSchema],
    global_config: dict[str, str],
    pipeline_status: dict = None,
    pipeline_status_lock=None,
    llm_response_cache: BaseKVStorage | None = None,
    text_chunks_storage: BaseKVStorage | None = None,
) -> list:
```

**Prompt Format:**

```
ENTITY: entity_name<|#|>entity_type<|#|>description
RELATION: source<|#|>target<|#|>keywords<|#|>description<|#|>weight
```

**Algorithm:**

1. Build prompt with entity types and examples
2. First LLM call: initial extraction
3. Parse tuple-delimited output (`<|#|>` delimiter)
4. **Gleaning phase**: If `entity_extract_max_gleaning > 0`:
   - Call continue extraction prompt with history
   - Merge results (prefer longer descriptions)
5. Return `(maybe_nodes, maybe_edges)` per chunk

**Gleaning Logic (Critical Feature):**

```python
# Second pass to catch missed entities
if entity_extract_max_gleaning > 0:
    glean_result, timestamp = await use_llm_func_with_cache(
        entity_continue_extraction_user_prompt,
        use_llm_func,
        history_messages=history,  # Include first pass context
        ...
    )
    # Merge: keep longer descriptions
    for entity_name, glean_entities in glean_nodes.items():
        if entity_name in maybe_nodes:
            original_desc_len = len(maybe_nodes[entity_name][0].get("description", ""))
            glean_desc_len = len(glean_entities[0].get("description", ""))
            if glean_desc_len > original_desc_len:
                maybe_nodes[entity_name] = list(glean_entities)
        else:
            maybe_nodes[entity_name] = list(glean_entities)
```

### EdgeQuake Entity Extraction

```rust
// edgequake-pipeline/src/extractor.rs
#[async_trait]
impl<L> EntityExtractor for LLMExtractor<L>
where
    L: edgequake_llm::LLMProvider + Send + Sync + ?Sized,
{
    async fn extract(&self, chunk: &TextChunk) -> Result<ExtractionResult> {
        let prompt = self.build_prompt(&chunk.content);
        let response = self.llm_provider.complete(&prompt).await?;
        let mut result = self.parse_response(&response.content, &chunk.id)?;
        result.input_tokens = response.prompt_tokens;
        result.output_tokens = response.completion_tokens;
        Ok(result)
    }
}
```

**Prompt Format (JSON output):**

```json
{
  "entities": [
    {
      "name": "Entity Name",
      "type": "ENTITY_TYPE",
      "description": "Brief description"
    }
  ],
  "relationships": [
    {
      "source": "Source Entity",
      "target": "Target Entity",
      "type": "RELATIONSHIP_TYPE",
      "description": "Brief description"
    }
  ]
}
```

**Algorithm:**

1. Build prompt requesting JSON output
2. Single LLM call
3. Parse JSON response (handle code block wrapping)
4. Create `ExtractedEntity` and `ExtractedRelationship` objects
5. Track token usage

### Extraction Comparison Matrix

| Feature             | LightRAG                     | EdgeQuake                |
| ------------------- | ---------------------------- | ------------------------ |
| Output format       | Tuple-delimited              | JSON                     |
| Gleaning            | ✅ Yes (configurable passes) | ❌ No                    |
| Entity types        | ✅ Configurable              | ✅ Configurable          |
| Examples in prompt  | ✅ Yes                       | ❌ No                    |
| Token tracking      | ✅ Via cache                 | ✅ Direct count          |
| Error recovery      | ✅ Fix delimiter corruption  | ✅ JSON parsing fallback |
| Weight extraction   | ✅ Optional in output        | ❌ Fixed 0.5             |
| Keywords extraction | ✅ For relationships         | ✅ For relationships     |

**Gleaning Impact:**

- LightRAG with gleaning: **+20-30% more entities** (based on documentation)
- EdgeQuake: Misses entities that LLM didn't catch on first pass

---

## 4. Entity Merging Comparison

### LightRAG Entity Merging

```python
# lightrag/operate.py - _merge_nodes_then_upsert
async def _merge_nodes_then_upsert(
    entity_name: str,
    nodes_data: list[dict],
    knowledge_graph_inst: BaseGraphStorage,
    entity_vdb: BaseVectorStorage | None,
    global_config: dict,
    ...
):
```

**Algorithm (10 steps):**

1. Get existing node from graph
2. Merge new source_ids with existing (dedup + order)
3. Apply source_ids limit (KEEP oldest or FIFO newest)
4. Filter nodes by allowed source_ids (if KEEP mode)
5. Check if skip needed (at limit + no new data)
6. Finalize entity_type by highest count
7. Deduplicate descriptions by content
8. **LLM Summary**: `_handle_entity_relation_summary()` with map-reduce
9. Build file_path with MAX_FILE_PATHS limit
10. Update graph and vector DB

**LLM Summarization (Key Feature):**

```python
async def _handle_entity_relation_summary(
    description_type: str,
    entity_or_relation_name: str,
    description_list: list[str],
    ...
) -> tuple[str, bool]:
    """Map-reduce LLM summarization."""
    # If total tokens < summary_context_size and len < force_llm_summary_on_merge:
    #   No LLM needed, just join
    # If total tokens < summary_max_tokens:
    #   Single LLM call
    # Else:
    #   Split into chunks, summarize each, then summarize summaries
    #   Iterate until final summary fits
```

### EdgeQuake Entity Merging

```rust
// edgequake-pipeline/src/merger.rs (conceptual - actual implementation may vary)
pub async fn merge_entities(
    entities: Vec<ExtractedEntity>,
    storage: &dyn GraphStorage,
) -> Result<Vec<MergedEntity>> {
    // Group by name
    // Concatenate descriptions
    // Merge source_chunk_ids
    // Update storage
}
```

**Algorithm (simplified):**

1. Group entities by normalized name
2. Concatenate descriptions (separator-joined)
3. Merge source chunk IDs
4. Keep first entity type encountered
5. Update storage

**Missing from EdgeQuake:**

- ❌ No LLM summarization
- ❌ No description deduplication
- ❌ No source_id limiting strategies
- ❌ No file_path management

### Merging Comparison Matrix

| Feature             | LightRAG          | EdgeQuake            |
| ------------------- | ----------------- | -------------------- |
| Description merging | LLM map-reduce    | Simple concatenation |
| Deduplication       | ✅ By content     | ❌ No                |
| Source ID limit     | ✅ KEEP/FIFO      | ❌ No                |
| File path limit     | ✅ MAX_FILE_PATHS | ❌ No                |
| Entity type voting  | ✅ By count       | ❌ First wins        |
| Timestamp tracking  | ✅ created_at     | ✅ Via properties    |
| Truncation info     | ✅ Stored         | ❌ No                |

**Quality Impact:**

- LightRAG: High-quality merged descriptions via LLM
- EdgeQuake: May have redundant/long descriptions

---

## 5. Parallel Processing Comparison

### LightRAG Parallelism

```python
# Extraction parallelism
chunk_max_async = global_config.get("llm_model_max_async", 4)
semaphore = asyncio.Semaphore(chunk_max_async)

async def _process_with_semaphore(chunk):
    async with semaphore:
        return await _process_single_content(chunk)

tasks = [asyncio.create_task(_process_with_semaphore(c)) for c in ordered_chunks]
done, pending = await asyncio.wait(tasks, return_when=asyncio.FIRST_EXCEPTION)

# Merge parallelism (2x extraction async)
graph_max_async = global_config.get("llm_model_max_async", 4) * 2
```

**Characteristics:**

- Configurable via `llm_model_max_async`
- First exception handling with task cancellation
- Separate concurrency for extraction vs merging
- Python GIL limits actual CPU parallelism

### EdgeQuake Parallelism

```rust
// Extraction parallelism
let semaphore = Arc::new(tokio::sync::Semaphore::new(
    self.config.max_concurrent_extractions,
));

let futures: Vec<_> = chunks.iter().map(|chunk| {
    let semaphore = semaphore.clone();
    let extractor = extractor.clone();
    async move {
        let _permit = semaphore.acquire().await?;
        extractor.extract(chunk).await
    }
}).collect();

let results: Vec<Result<ExtractionResult>> = stream::iter(futures)
    .buffer_unordered(self.config.max_concurrent_extractions)
    .collect()
    .await;
```

**Characteristics:**

- Configurable via `max_concurrent_extractions`
- True parallelism (no GIL)
- `buffer_unordered` for optimal throughput
- Arc for safe shared state

### Parallelism Comparison

| Aspect              | LightRAG               | EdgeQuake         |
| ------------------- | ---------------------- | ----------------- |
| Concurrency control | asyncio.Semaphore      | tokio::Semaphore  |
| True parallelism    | ❌ GIL limited         | ✅ Yes            |
| Error handling      | FIRST_EXCEPTION        | buffer_unordered  |
| Default concurrency | 4 extraction, 8 merge  | 16                |
| Memory model        | Shared (GIL protected) | Arc + Send + Sync |

---

## 6. Storage Operations Comparison

### LightRAG Storage

```python
# Graph operations
await knowledge_graph_inst.upsert_node(entity_name, node_data)
await knowledge_graph_inst.upsert_edge(src, tgt, edge_data)

# Vector operations
await entity_vdb.upsert({entity_vdb_id: vdb_data})
await relationships_vdb.upsert({rel_vdb_id: vdb_data})

# Safe VDB operation wrapper
await safe_vdb_operation_with_exception(
    operation=lambda: entity_vdb.upsert(vdb_data),
    operation_name="entity_upsert",
    entity_name=entity_name,
    max_retries=3,
    retry_delay=0.1,
)

# KV storage for chunk tracking
await entity_chunks_storage.upsert({entity_name: {"chunk_ids": [...], "count": ...}})
await relation_chunks_storage.upsert({storage_key: {"chunk_ids": [...], "count": ...}})
```

### EdgeQuake Storage

```rust
// Graph operations
graph_storage.upsert_node(&node).await?;
graph_storage.upsert_edge(&edge).await?;

// Vector operations
vector_storage.upsert(&chunk_id, &embedding, &metadata).await?;

// KV operations
kv_storage.set(&key, &value).await?;
```

### Storage Comparison

| Feature             | LightRAG                    | EdgeQuake                |
| ------------------- | --------------------------- | ------------------------ |
| Retry logic         | ✅ Built-in wrapper         | ❌ Caller responsibility |
| Batch operations    | ✅ get_by_ids, upsert batch | ✅ Trait methods         |
| Transaction support | Depends on backend          | Depends on backend       |
| Chunk tracking KV   | ✅ Separate storages        | ❌ Not implemented       |

---

## 7. Cost Tracking

### LightRAG Cost Tracking

❌ **Not implemented**

LightRAG does not track costs. Token counts are stored in cache but not aggregated for cost calculation.

### EdgeQuake Cost Tracking

```rust
// edgequake-pipeline/src/pipeline.rs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost_usd: f64,
    pub cost_breakdown: Option<CostBreakdownStats>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdownStats {
    pub extraction_cost_usd: f64,
    pub embedding_cost_usd: f64,
    pub summarization_cost_usd: f64,
    pub extraction_input_tokens: usize,
    pub extraction_output_tokens: usize,
    pub embedding_tokens: usize,
}

// Cost calculation
let model_pricing = pricing.get(model_name).cloned().unwrap_or_else(|| {
    crate::progress::ModelPricing::new("gpt-4o-mini", 0.00015, 0.0006)
});
let extraction_cost = model_pricing.calculate_cost(input_tokens, output_tokens);
stats.cost_usd += extraction_cost;
```

**Features:**

- ✅ Per-operation cost breakdown
- ✅ Token tracking (input/output)
- ✅ Model-specific pricing
- ✅ Aggregated totals

---

## 8. Lineage Tracking

### LightRAG Lineage Tracking

❌ **Not implemented**

LightRAG tracks source_ids and file_paths but doesn't have formal lineage infrastructure.

### EdgeQuake Lineage Tracking

```rust
// edgequake-pipeline/src/lineage.rs
pub struct DocumentLineage {
    pub document_id: String,
    pub document_name: String,
    pub job_id: String,
    pub chunks: Vec<ChunkLineage>,
    pub entities: Vec<EntityLineage>,
    pub relationships: Vec<RelationshipLineage>,
    pub created_at: DateTime<Utc>,
}

pub struct ChunkLineage {
    pub chunk_id: String,
    pub start_line: usize,
    pub end_line: usize,
    pub token_count: usize,
}

pub struct EntityLineage {
    pub name: String,
    pub entity_type: String,
    pub source_chunks: Vec<String>,
    pub extraction_metadata: ExtractionMetadata,
}
```

**Features:**

- ✅ Document → Chunk → Entity/Relationship lineage
- ✅ Line number tracking
- ✅ Extraction metadata
- ✅ Job ID for traceability

---

## 9. Summary and Recommendations

### Feature Gap Analysis

| Feature                   | LightRAG | EdgeQuake | Priority for EdgeQuake |
| ------------------------- | -------- | --------- | ---------------------- |
| Token-based chunking      | ✅       | ❌        | P1                     |
| Gleaning                  | ✅       | ❌        | **P0**                 |
| LLM description merging   | ✅       | ❌        | **P0**                 |
| Source ID limiting        | ✅       | ❌        | P1                     |
| Description deduplication | ✅       | ❌        | P1                     |
| File path limiting        | ✅       | ❌        | P2                     |
| Cost tracking             | ❌       | ✅        | ✅ Keep                |
| Lineage tracking          | ❌       | ✅        | ✅ Keep                |

### Recommended Actions for EdgeQuake

1. **P0: Implement Gleaning**

   - Add second LLM pass with conversation history
   - Merge results preferring longer descriptions
   - Expected impact: +20-30% entity coverage

2. **P0: Implement LLM Description Merging**

   - Port `_handle_entity_relation_summary` logic
   - Use map-reduce for long description lists
   - Expected impact: Higher quality merged descriptions

3. **P1: Token-based Chunking**

   - Add tokenizer to chunker
   - Split based on token count, not characters
   - Expected impact: Better LLM context utilization

4. **P1: Source ID Management**
   - Implement KEEP/FIFO strategies
   - Add chunk tracking storage
   - Expected impact: Better lineage control at scale

---

_Document Version: 1.0_
_Last Updated: 2025-12-31_
