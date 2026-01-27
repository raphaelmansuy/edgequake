# OODA-230 Iteration 224: Decide

**Date**: January 16, 2026
**Decision**: Fix Missing source_chunk_ids Logic in Workspace Vector Methods

## Decision Matrix

| Option                   | Complexity | Risk   | Time  | Chosen |
| ------------------------ | ---------- | ------ | ----- | ------ |
| A: Copy chunk logic      | Low        | Low    | 30min | ✓      |
| B: Extract shared method | Medium     | Medium | 1hr   |        |

## Implementation Plan

### Step 1: Fix `query_local_with_vector_storage`

Add the following chunk collection logic after entity/relationship retrieval:

```rust
// Step 7: Collect source_chunk_ids from entities and relationships
// WHY-OODA230: Must retrieve chunks via their IDs, not by semantic similarity
let mut chunk_ids = std::collections::HashSet::new();

// Collect chunk IDs from entities
for entity in &context.entities {
    for chunk_id in &entity.source_chunk_ids {
        chunk_ids.insert(chunk_id.clone());
    }
}

// Collect chunk IDs from relationships
for rel in &context.relationships {
    if let Some(chunk_id) = &rel.source_chunk_id {
        chunk_ids.insert(chunk_id.clone());
    }
}

tracing::info!(
    total_chunk_ids = chunk_ids.len(),
    entity_count = context.entities.len(),
    "OODA-230: Local mode chunk collection (workspace)"
);

// Retrieve chunks from workspace vector storage using chunk IDs
if !chunk_ids.is_empty() {
    let chunk_ids_vec: Vec<String> = chunk_ids.into_iter()
        .take(self.config.max_chunks)
        .collect();

    // Query with filter to retrieve only the specific chunks
    let results = vector_storage
        .query(
            &embeddings.low_level,
            chunk_ids_vec.len(),
            Some(&chunk_ids_vec),
        )
        .await?;

    for result in results {
        if !self.matches_tenant_filter(&result.metadata, &tenant_id, &workspace_id) {
            continue;
        }
        context.add_chunk(build_chunk_from_result(&result));
    }
}
```

### Step 2: Fix `query_global_with_vector_storage`

Same fix pattern applied to global mode.

### Step 3: Add E2E Test

Create test that verifies chunk retrieval with workspace vector storage:

```rust
#[tokio::test]
async fn test_hybrid_query_returns_chunks_with_workspace_storage() {
    // Setup: Create workspace with OpenAI 3072-dim embeddings
    // Ingest: Store document with chunks
    // Query: Execute Hybrid mode query
    // Assert: context.chunks is NOT empty
}
```

### Step 4: Regression Test

Run full test suite to ensure no regressions.

## Verification Criteria

| Criterion                      | Test Method                              |
| ------------------------------ | ---------------------------------------- |
| Chunks returned in Hybrid mode | Query returns `context.chunks.len() > 0` |
| Chunks returned in Local mode  | Query returns `context.chunks.len() > 0` |
| Chunks returned in Global mode | Query returns `context.chunks.len() > 0` |
| Existing tests pass            | `cargo test --package edgequake-query`   |
| User scenario works            | Manual test with OpenAI workspace        |

## Risk Mitigation

1. **Test Before Deploy**: Run full E2E test suite
2. **Logging**: Add explicit logging for chunk collection
3. **Rollback Plan**: Revert commit if issues found

## Acceptance Criteria

- [ ] `query_local_with_vector_storage` collects source_chunk_ids
- [ ] `query_global_with_vector_storage` collects source_chunk_ids
- [ ] New E2E test covers chunk retrieval
- [ ] All existing tests pass
- [ ] `cargo clippy` clean
