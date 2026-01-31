# Iteration 07: Observe - Pipeline Architecture Deep Dive

## Topic: Document Processing Pipeline Architecture

### Codebase Research Results

#### Pipeline Module Structure (`edgequake-pipeline/src/lib.rs`)

```
Pipeline Stages:
├── Chunking (FEAT0004)     → Split documents into overlapping chunks
├── Entity Extraction (FEAT0002) → Use LLM to extract entities
├── Relationship Extraction (FEAT0003) → Use LLM to extract relationships
├── Merging (FEAT0006)      → Deduplicate and merge into graph
└── Embedding (FEAT0005)    → Generate and store embeddings
```

#### Key Business Rules Discovered:

- **BR0002**: Chunk size 1200 tokens, overlap 100 tokens
- **BR0003**: Entity types from configurable list
- **BR0004**: Relationship keywords max 5 per edge
- **BR0005**: Entity description max 512 tokens
- **BR0006**: Same-entity relationships forbidden
- **BR0008**: Entity names normalized (UPPERCASE_UNDERSCORE)

#### Pipeline Configuration (`pipeline.rs`)

```rust
PipelineConfig {
    chunk_size: 1200,           // tokens per chunk
    chunk_overlap: 100,         // ~8% overlap
    extraction_batch_size: 10,
    embedding_batch_size: 100,
    max_concurrent_extractions: 16,  // semaphore-controlled
    chunk_extraction_timeout_secs: 60,
    chunk_max_retries: 3,
    initial_retry_delay_ms: 1000,    // exponential backoff
}
```

#### Resilient Extraction Architecture (MAP-REDUCE Pattern)

From pipeline.rs lines 537-620:

```
MAP PHASE:
  Document (N chunks)
       │
       ▼
  ┌────┬────┬────┬────┬────┐
  │ C1 │ C2 │ C3 │ C4 │ CN │   (chunks distributed to workers)
  └─┬──┴─┬──┴─┬──┴─┬──┴─┬──┘
    │    │    │    │    │
    ▼    ▼    ▼    ▼    ▼      (parallel LLM calls with semaphore)
  ┌───┐┌───┐┌───┐┌───┐┌───┐
  │ E ││ E ││ E ││ E ││ E │    (each E = extract_with_retry)
  └─┬─┘└─┬─┘└─┬─┘└─┬─┘└─┬─┘
    │    │    │    │    │
    ▼    ▼    ▼    ▼    ▼
  ┌───┐┌───┐┌───┐┌───┐┌───┐
  │ ✓ ││ ✗ ││ ✓ ││ ✓ ││ ✓ │    (✓ = Success, ✗ = Failed)
  └───┘└───┘└───┘└───┘└───┘

REDUCE PHASE:
  - Partition: successes = [C1, C3, C4, CN], failures = [C2]
  - Sort by chunk_index (maintain document order)
  - Calculate stats: 4/5 = 80% success rate
```

#### Chunking Strategies (`chunker.rs`)

- **CharacterBasedChunking**: Split on character boundaries
- **TokenBasedChunking**: Respect token limits for LLM context
- **SentenceBoundaryChunking**: Preserve sentence structure
- **ParagraphBoundaryChunking**: Split on paragraphs

Why overlap (100 tokens, ~8%):

1. Context continuity across chunk boundaries
2. Entity mentions spanning two chunks are captured
3. Better retrieval for queries at chunk boundaries

#### Extraction Strategies (`extractor.rs`)

- **SOTAExtractor**: Tuple-based parsing (production - robust)
- **SimpleExtractor**: JSON-based parsing (development)
- **GleaningExtractor**: Iterative re-extraction (high-stakes)

#### Merger Logic (`merger.rs`)

Why merge, don't replace:

1. **Names match** (after normalization): Same graph node
2. **Descriptions merge**: Combine via LLM summarization
3. **Sources accumulate**: `source_id` = "chunk1|chunk2|chunk3"

This strategy:

- Builds richer entity descriptions over time
- Maintains full provenance for source tracking
- Enables cascade delete via source_id filtering

### Key Differentiators

1. **Resilient extraction**: Partial success, not fail-fast
2. **Concurrent processing**: Semaphore + Tokio async
3. **Real-time progress**: Per-chunk callbacks with ETA
4. **Cost tracking**: Token/cost calculation per operation
5. **Lineage tracking**: Full document → chunk → entity traceability
