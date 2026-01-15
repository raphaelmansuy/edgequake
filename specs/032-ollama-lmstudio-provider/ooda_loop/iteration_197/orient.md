# OODA 197: Orient - Provider Lineage Implementation Design

**Date**: 2025-01-15
**Focus**: Designing provider lineage tracking implementation

## Decision: Option 3 - Both Metadata and Lineage

### Rationale

1. **Quick Access**: Document metadata is returned with every document GET request
2. **Detailed Traceability**: DocumentLineage provides complete history
3. **Backward Compatible**: Adding to metadata doesn't break existing code

## Implementation Plan

### Phase 1: Extend DocumentLineage

Add fields to [lineage.rs](../../../../edgequake/crates/edgequake-pipeline/src/lineage.rs):

```rust
pub struct DocumentLineage {
    // ... existing fields ...

    /// SPEC-032: Provider used for entity extraction (LLM)
    pub extraction_provider: Option<String>,
    /// SPEC-032: Model used for entity extraction
    pub extraction_model: Option<String>,
    /// SPEC-032: Provider used for embedding generation
    pub embedding_provider: Option<String>,
    /// SPEC-032: Model used for embedding generation
    pub embedding_model: Option<String>,
    /// SPEC-032: Embedding dimension used
    pub embedding_dimension: Option<usize>,
}
```

### Phase 2: Capture in Processor

Modify [processor.rs](../../../../edgequake/crates/edgequake-api/src/processor.rs) to:

1. Extract workspace config before pipeline call
2. Pass provider info through pipeline
3. Store in result metadata

```rust
// In get_workspace_pipeline()
let provider_info = ProviderLineage {
    llm_provider: ws.llm_provider.clone(),
    llm_model: ws.llm_model.clone(),
    embedding_provider: ws.embedding_provider.clone(),
    embedding_model: ws.embedding_model.clone(),
};

// Store in document metadata
metadata["provider_lineage"] = json!({
    "extraction_provider": provider_info.llm_provider,
    "extraction_model": provider_info.llm_model,
    "embedding_provider": provider_info.embedding_provider,
    "embedding_model": provider_info.embedding_model,
});
```

### Phase 3: API Exposure

Add to [documents_types.rs](../../../../edgequake/crates/edgequake-api/src/handlers/documents_types.rs):

```rust
#[derive(Serialize, Deserialize)]
pub struct ProviderLineage {
    pub extraction_provider: String,
    pub extraction_model: String,
    pub embedding_provider: String,
    pub embedding_model: String,
}

// Include in DocumentDetailResponse
pub struct DocumentDetailResponse {
    // ... existing fields ...
    pub provider_lineage: Option<ProviderLineage>,
}
```

### Phase 4: UI Display

Already captured in messages - need to verify display in UI.

## Files to Modify

1. `/crates/edgequake-pipeline/src/lineage.rs` - Add provider fields
2. `/crates/edgequake-api/src/processor.rs` - Capture and store provider info
3. `/crates/edgequake-api/src/handlers/documents_types.rs` - API types
4. `/crates/edgequake-api/src/handlers/documents.rs` - Return lineage in API

## Risk Assessment

| Risk             | Mitigation                   |
| ---------------- | ---------------------------- |
| Breaking changes | Use Option<T> for new fields |
| Performance      | Minimal - just string copies |
| Storage increase | ~100 bytes per document      |

## Next Step

OODA 198: Decide - Create implementation tasks
