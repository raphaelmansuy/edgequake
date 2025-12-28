# Comparison: EdgeQuake (Rust) vs LightRAG (Python)

> Document ID: COMP-001
> Version: 1.0
> Created: 2024-12-28

## Executive Summary

This document provides a detailed comparison between the EdgeQuake Rust implementation and the LightRAG Python implementation, highlighting architectural differences, feature parity, and recommendations for the SOTA ingestion pipeline.

---

## 1. Implementation Comparison Matrix

### 1.1 Core Pipeline Features

| Feature | EdgeQuake (Rust) | LightRAG (Python) | Gap Analysis |
|---------|------------------|-------------------|--------------|
| **Document Chunking** | ✅ Token-based with overlap | ✅ Token-based with overlap | Parity |
| **Character-based Split** | ✅ (GAP-017) | ✅ | Parity |
| **Line Number Tracking** | ❌ Char offsets only | ❌ Not tracked | Both need enhancement |
| **Entity Extraction** | ✅ LLM-based | ✅ LLM-based | Parity |
| **Relationship Extraction** | ✅ Same-pass as entities | ✅ Same-pass | Parity |
| **Gleaning/Re-extraction** | ✅ (GAP-018) | ✅ Continue extraction | Parity |
| **Description Summarization** | ✅ Basic | ✅ MapReduce | Rust needs enhancement |
| **Deduplication** | ✅ Name normalization | ✅ Name normalization | Parity |
| **Description Merging** | ✅ Keep longer | ✅ Keep longer + LLM merge | Python more advanced |

### 1.2 Storage & Persistence

| Feature | EdgeQuake (Rust) | LightRAG (Python) | Gap Analysis |
|---------|------------------|-------------------|--------------|
| **Graph Storage** | ✅ Apache AGE/Memory | ✅ Neo4j/NetworkX | Different backends |
| **Vector Storage** | ✅ pgvector/Memory | ✅ Milvus/ChromaDB/Qdrant | Different backends |
| **KV Storage** | ✅ PostgreSQL/Memory | ✅ Various backends | Parity |
| **LLM Response Caching** | ⚠️ Basic | ✅ Comprehensive | Rust needs enhancement |
| **Chunk Cache Tracking** | ❌ Not tracked | ✅ llm_cache_list per chunk | Rust needs this |
| **Rebuild from Cache** | ❌ Not implemented | ✅ Full rebuild support | Rust needs this |

### 1.3 Multi-Tenancy & Isolation

| Feature | EdgeQuake (Rust) | LightRAG (Python) | Gap Analysis |
|---------|------------------|-------------------|--------------|
| **Tenant Isolation** | ✅ TenantManager | ✅ TenantRAGManager | Parity |
| **Workspace Support** | ✅ Per-workspace instances | ✅ Per-workspace | Parity |
| **Namespace-based Queries** | ✅ Filter by namespace | ✅ Filter by namespace | Parity |
| **Cross-namespace Query** | ❌ Not implemented | ❌ Not implemented | Future feature |

### 1.4 Progress & Monitoring

| Feature | EdgeQuake (Rust) | LightRAG (Python) | Gap Analysis |
|---------|------------------|-------------------|--------------|
| **Progress Tracking** | ⚠️ Basic stats | ✅ Detailed status | Rust needs enhancement |
| **History Messages** | ❌ Not tracked | ✅ history_messages list | Rust needs this |
| **Error Tracking** | ⚠️ Basic | ✅ Detailed error counts | Rust needs enhancement |
| **Stage-level Progress** | ❌ Not implemented | ✅ Per-stage tracking | Rust needs this |

### 1.5 Cost Management

| Feature | EdgeQuake (Rust) | LightRAG (Python) | Gap Analysis |
|---------|------------------|-------------------|--------------|
| **LLM Call Counting** | ✅ llm_calls field | ✅ Tracked | Parity |
| **Token Counting** | ✅ total_tokens | ✅ Input/output separate | Python more detailed |
| **Cost in USD** | ❌ Not tracked | ⚠️ Partial | Both need enhancement |
| **Per-operation Breakdown** | ❌ Not tracked | ❌ Not tracked | Both need enhancement |

---

## 2. Architectural Differences

### 2.1 Extraction Prompt Format

**EdgeQuake (Rust) - JSON-based:**
```
Extract entities and relationships from the following text.

## Output Format
Respond with valid JSON in this exact format:
{
  "entities": [...],
  "relationships": [...]
}
```

**LightRAG (Python) - Tuple-based:**
```
entity<|#|>entity_name<|#|>entity_type<|#|>entity_description
relation<|#|>source_entity<|#|>target_entity<|#|>keywords<|#|>description
<|COMPLETE|>
```

**Comparison:**
| Aspect | JSON (Rust) | Tuple (Python) | Winner |
|--------|-------------|----------------|--------|
| Parse reliability | Lower (LLM may malform JSON) | Higher (simpler parsing) | Python |
| Human readability | Higher | Lower | Rust |
| Token efficiency | Lower (more syntax overhead) | Higher (less overhead) | Python |
| Schema flexibility | Higher (nested structures) | Lower | Rust |

**Recommendation:** Consider hybrid approach - use tuple format for extraction, JSON for complex operations.

### 2.2 Description Summarization

**EdgeQuake (Rust):**
```rust
fn merge_descriptions(existing: &str, new: &str, max_length: usize) -> String {
    // Simple concatenation with separator
    // Truncate if exceeds max_length
}
```

**LightRAG (Python):**
```python
async def _handle_entity_relation_summary():
    # 1. If total_tokens < limit: just concatenate
    # 2. If exceeds: use MapReduce
    #    - Split descriptions into chunks
    #    - LLM summarize each chunk
    #    - Recursively summarize summaries
    # 3. Return final summary
```

**Comparison:**
| Aspect | Rust (Simple) | Python (MapReduce) | Winner |
|--------|---------------|-------------------|--------|
| Quality | Lower (truncation) | Higher (intelligent merge) | Python |
| Performance | Higher (no LLM call) | Lower (LLM calls) | Rust |
| Cost | Lower | Higher | Rust |
| Scalability | Poor (fixed limit) | Good (handles any size) | Python |

**Recommendation:** Implement MapReduce summarization in Rust with configurable threshold.

### 2.3 Caching Strategy

**EdgeQuake (Rust):**
```rust
// Basic cache check (not fully implemented)
pub enable_cache: bool,
```

**LightRAG (Python):**
```python
# Comprehensive caching
chunk_data = {
    "content": content,
    "llm_cache_list": [cache_id1, cache_id2, ...],  # Links to LLM cache entries
    "tokens": token_count,
}

llm_cache_entry = {
    "cache_type": "extract",
    "chunk_id": chunk_id,
    "return": extraction_result,
    "create_time": timestamp,
}
```

**Recommendation:** Implement comprehensive caching in Rust following Python pattern.

---

## 3. Feature Comparison Deep Dive

### 3.1 Entity/Relationship Extraction

```
┌────────────────────────────────────────────────────────────────────────┐
│                    EXTRACTION FLOW COMPARISON                          │
└────────────────────────────────────────────────────────────────────────┘

EdgeQuake (Rust):
─────────────────
  Chunk → LLMExtractor → JSON Parse → ExtractedEntity/Relationship
                ↓
         GleaningExtractor (optional)
                ↓
         Merge with existing

LightRAG (Python):
──────────────────
  Chunk → extract_entities (tuple format) → Parse tuples
                ↓
         Continue extraction (if max_gleaning > 0)
                ↓
         Merge with cache check
                ↓
         _handle_entity_relation_summary (if descriptions long)
                ↓
         Store with llm_cache_list reference
```

### 3.2 Keyword Extraction

**EdgeQuake (Rust):**
- Keywords extracted as part of relationship
- Stored in `ExtractedRelationship.keywords`

**LightRAG (Python):**
- Separate keyword extraction for queries
- `PROMPTS["keywords_extraction"]` for query processing
- High-level and low-level keyword separation

**Recommendation:** Add separate keyword extraction for improved query understanding.

### 3.3 Parallel Processing

**EdgeQuake (Rust):**
```rust
// Sequential processing in process()
for chunk in &chunks {
    let extraction = extractor.extract(chunk).await?;
    // ...
}
```

**LightRAG (Python):**
```python
# Parallel processing with semaphore control
graph_max_async = global_config.get("llm_model_max_async", 4) * 2
semaphore = asyncio.Semaphore(graph_max_async)

tasks = []
for chunk_id in chunks:
    task = asyncio.create_task(_process_chunk(chunk_id))
    tasks.append(task)

await asyncio.wait(tasks)
```

**Recommendation:** Implement parallel chunk processing in Rust using tokio.

---

## 4. Pros and Cons

### 4.1 EdgeQuake (Rust)

**Pros:**
- ✅ Type safety and compile-time guarantees
- ✅ Memory safety without garbage collection
- ✅ Lower runtime overhead
- ✅ Better async/await with tokio
- ✅ Modular crate structure
- ✅ Strong API with Axum
- ✅ Native multi-tenancy support

**Cons:**
- ❌ Less mature extraction logic
- ❌ No MapReduce for descriptions
- ❌ Limited caching implementation
- ❌ Sequential processing only
- ❌ No line number tracking
- ❌ No rebuild from cache

### 4.2 LightRAG (Python)

**Pros:**
- ✅ Mature extraction with tuple format
- ✅ MapReduce description summarization
- ✅ Comprehensive LLM caching
- ✅ Parallel processing
- ✅ Detailed progress tracking
- ✅ Rebuild from cache support
- ✅ Battle-tested in production

**Cons:**
- ❌ Higher memory usage
- ❌ GIL limitations for true parallelism
- ❌ Larger codebase (5000 lines in operate.py)
- ❌ Less type safety
- ❌ No native compilation

---

## 5. Recommendations

### 5.1 Features to Port from Python to Rust

| Priority | Feature | Effort | Impact |
|----------|---------|--------|--------|
| **P0** | MapReduce summarization | Medium | High |
| **P0** | Comprehensive caching | Medium | High |
| **P0** | Parallel chunk processing | Low | High |
| **P0** | Progress tracking enhancement | Low | Medium |
| **P1** | Tuple-based extraction format | Medium | Medium |
| **P1** | Keyword extraction for queries | Low | Medium |
| **P1** | Rebuild from cache | Medium | Medium |
| **P2** | Line number tracking | Low | Medium |
| **P2** | Cost breakdown tracking | Low | Low |

### 5.2 Features Unique to Rust to Preserve

| Feature | Reason to Preserve |
|---------|-------------------|
| Type-safe configurations | Compile-time validation |
| Trait-based storage abstraction | Flexibility |
| Modular crate structure | Maintainability |
| Axum-based API | Performance |

### 5.3 Hybrid Best Practices

1. **Extraction Format:** Use tuple format for reliability
2. **Summarization:** Implement MapReduce with configurable threshold
3. **Caching:** Link cache entries to chunks for rebuild
4. **Progress:** Stream events for real-time updates
5. **Parallelism:** Use tokio::spawn for concurrent extraction

---

## 6. Migration Path

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        MIGRATION ROADMAP                                │
└─────────────────────────────────────────────────────────────────────────┘

Phase 1: Core Enhancements (Week 1-2)
─────────────────────────────────────
  ├── Add line number tracking to TextChunk
  ├── Implement parallel chunk processing
  ├── Enhance progress tracking
  └── Add token usage breakdown

Phase 2: Caching & Rebuild (Week 2-3)
─────────────────────────────────────
  ├── Implement comprehensive LLM caching
  ├── Add llm_cache_list to chunks
  ├── Implement rebuild from cache
  └── Add cache invalidation

Phase 3: MapReduce (Week 3-4)
─────────────────────────────
  ├── Implement description MapReduce
  ├── Add configurable threshold
  ├── Implement recursive summarization
  └── Test with large documents

Phase 4: Advanced Features (Week 4+)
────────────────────────────────────
  ├── Tuple-based extraction format
  ├── Keyword extraction enhancement
  ├── Cost tracking enhancement
  └── Evaluation suite integration
```

---

## Appendix A: Code Examples

### A.1 Tuple-Based Extraction Parser (To Implement)

```rust
/// Parse tuple-based extraction output
pub fn parse_extraction_tuples(response: &str) -> Result<ExtractionResult> {
    let mut result = ExtractionResult::default();
    let delimiter = "<|#|>";
    let completion = "<|COMPLETE|>";
    
    for line in response.lines() {
        let line = line.trim();
        if line.is_empty() || line == completion {
            continue;
        }
        
        let parts: Vec<&str> = line.split(delimiter).collect();
        
        match parts.get(0).map(|s| s.to_lowercase()).as_deref() {
            Some("entity") if parts.len() >= 4 => {
                result.entities.push(ExtractedEntity {
                    name: normalize_entity_name(parts[1]),
                    entity_type: parts[2].to_string(),
                    description: parts[3].to_string(),
                    ..Default::default()
                });
            }
            Some("relation") if parts.len() >= 5 => {
                result.relationships.push(ExtractedRelationship {
                    source: normalize_entity_name(parts[1]),
                    target: normalize_entity_name(parts[2]),
                    keywords: parts[3].split(',').map(|s| s.trim().to_string()).collect(),
                    description: parts[4].to_string(),
                    ..Default::default()
                });
            }
            _ => {
                tracing::warn!("Unrecognized extraction line: {}", line);
            }
        }
    }
    
    Ok(result)
}
```

### A.2 MapReduce Summarization (To Implement)

```rust
/// MapReduce description summarization
pub async fn summarize_descriptions_mapreduce<L: LLMProvider>(
    llm: &L,
    descriptions: Vec<String>,
    config: &SummarizerConfig,
) -> Result<String> {
    // Base case: single description
    if descriptions.len() == 1 {
        return Ok(descriptions[0].clone());
    }
    
    // Calculate total tokens
    let total_tokens: usize = descriptions.iter()
        .map(|d| estimate_tokens(d))
        .sum();
    
    // If within limit, concatenate
    if total_tokens <= config.context_size && descriptions.len() < config.force_llm_threshold {
        return Ok(descriptions.join("\n\n"));
    }
    
    // MAP: Split into chunks and summarize each
    let chunks = split_into_chunks(&descriptions, config.context_size);
    let mut summaries = Vec::new();
    
    for chunk in chunks {
        if chunk.len() == 1 {
            summaries.push(chunk[0].clone());
        } else {
            let summary = llm.summarize(&chunk.join("\n\n")).await?;
            summaries.push(summary);
        }
    }
    
    // REDUCE: Recursively summarize summaries
    Box::pin(summarize_descriptions_mapreduce(llm, summaries, config)).await
}
```

---
