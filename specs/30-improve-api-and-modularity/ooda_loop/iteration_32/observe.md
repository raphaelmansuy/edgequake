# Iteration 32 - Observe

**Date:** 2026-01-07  
**Focus:** query.rs DTO extraction  
**Commit:** c35cb55

## Current State

### File Analysis

- **query.rs:** 588 lines
  - Handler functions for query execution and streaming
  - 6 inline DTOs: ConversationMessage, QueryRequest, QueryResponse, SourceReference, QueryStats, StreamQueryRequest
  - 1 helper function: default_enable_rerank()
  - OpenAPI documentation with utoipa annotations

### DTO Identification

```rust
// Line 16
pub struct ConversationMessage { role, content }

// Line 26
pub struct QueryRequest {
    query, mode, context_only, prompt_only, include_references,
    max_results, conversation_history, enable_rerank, rerank_model, rerank_top_k
}

// Line 74
pub struct QueryResponse {
    answer, mode, sources, stats, conversation_id, reranked
}

// Line 98
pub struct SourceReference {
    source_type, id, score, rerank_score, snippet, reference_id,
    document_id, file_path, start_line, end_line, chunk_index
}

// Line 142
pub struct QueryStats {
    embedding_time_ms, retrieval_time_ms, generation_time_ms,
    total_time_ms, sources_retrieved, rerank_time_ms
}

// Line 373
pub struct StreamQueryRequest { query, mode }

// Helper function
fn default_enable_rerank() -> bool { true }
```

### Dependencies

- Used by chat.rs and chat_types.rs (imports QueryStats, SourceReference)
- Core query execution and streaming logic
- Integration with edgequake-query engine

### Test Coverage

- **Before:** 252 tests passing
- **Pattern:** Sibling query_types.rs with 8-10 unit tests

## Metrics

| Metric             | Value     |
| ------------------ | --------- |
| File size          | 588 lines |
| DTOs to extract    | 6         |
| Helper functions   | 1         |
| Expected reduction | ~27%      |
| Current test count | 252       |
| Expected new tests | +9        |

## Risk Assessment

**Low Risk:**

- Established pattern (7th extraction this session)
- All previous extractions successful
- Clean DTO structure with no circular dependencies

**Considerations:**

- QueryStats and SourceReference imported by other handlers
- Must re-export for backward compatibility
- Floating point comparisons in tests need epsilon checks
