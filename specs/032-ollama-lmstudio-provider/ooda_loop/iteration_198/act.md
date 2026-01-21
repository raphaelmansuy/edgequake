# OODA 198-199: Act - Provider Lineage Implementation

**Date**: 2025-01-15
**Focus**: Implementing provider lineage tracking in document processing

## Changes Made

### 1. Extended ProcessingStats

**File**: [pipeline.rs#L103-180](../../../../edgequake/crates/edgequake-pipeline/src/pipeline.rs#L103)

Added two new fields:

```rust
/// SPEC-032/OODA-198: LLM provider used for entity extraction.
#[serde(skip_serializing_if = "Option::is_none")]
pub llm_provider: Option<String>,

/// SPEC-032/OODA-198: Embedding provider used for vector embeddings.
#[serde(skip_serializing_if = "Option::is_none")]
pub embedding_provider: Option<String>,
```

### 2. Extended DocumentLineage

**File**: [lineage.rs#L334-403](../../../../edgequake/crates/edgequake-pipeline/src/lineage.rs#L334)

Added provider lineage fields:

```rust
pub extraction_provider: Option<String>,
pub extraction_model: Option<String>,
pub embedding_provider: Option<String>,
pub embedding_model: Option<String>,
pub embedding_dimension: Option<usize>,
```

Added method:

```rust
pub fn set_provider_lineage(
    &mut self,
    extraction_provider: impl Into<String>,
    extraction_model: impl Into<String>,
    embedding_provider: impl Into<String>,
    embedding_model: impl Into<String>,
    embedding_dimension: usize,
)
```

### 3. Added ProviderLineage Struct

**File**: [processor.rs#L37-52](../../../../edgequake/crates/edgequake-api/src/processor.rs#L37)

```rust
#[derive(Debug, Clone, Default)]
pub struct ProviderLineage {
    pub extraction_provider: String,
    pub extraction_model: String,
    pub embedding_provider: String,
    pub embedding_model: String,
    pub embedding_dimension: usize,
}
```

### 4. Added get_workspace_provider_lineage Method

**File**: [processor.rs#L376-429](../../../../edgequake/crates/edgequake-api/src/processor.rs#L376)

Retrieves workspace provider configuration for lineage tracking.

### 5. Updated process_text_insert

**File**: [processor.rs#L462-472](../../../../edgequake/crates/edgequake-api/src/processor.rs#L462)

Now captures provider lineage and logs it:

```rust
let provider_lineage = self.get_workspace_provider_lineage(workspace_id).await;
info!(
    document_id = %document_id,
    extraction_provider = %provider_lineage.extraction_provider,
    extraction_model = %provider_lineage.extraction_model,
    embedding_provider = %provider_lineage.embedding_provider,
    "Processing document with workspace-specific pipeline"
);
```

### 6. Augmented Stats Before Storage

**File**: [processor.rs#L717-723](../../../../edgequake/crates/edgequake-api/src/processor.rs#L717)

```rust
let mut stats_with_lineage = result.stats.clone();
stats_with_lineage.llm_provider = Some(provider_lineage.extraction_provider.clone());
stats_with_lineage.llm_model = Some(provider_lineage.extraction_model.clone());
stats_with_lineage.embedding_provider = Some(provider_lineage.embedding_provider.clone());
stats_with_lineage.embedding_model = Some(provider_lineage.embedding_model.clone());
stats_with_lineage.embedding_dimensions = Some(provider_lineage.embedding_dimension);
```

### 7. Updated Metadata Storage

**File**: [processor.rs#L822-834](../../../../edgequake/crates/edgequake-api/src/processor.rs#L822)

Added storage of provider fields in document metadata:

```rust
if let Some(ref llm_provider) = stats.llm_provider {
    updated.insert("llm_provider".to_string(), json!(llm_provider));
}
if let Some(ref embedding_provider) = stats.embedding_provider {
    updated.insert("embedding_provider".to_string(), json!(embedding_provider));
}
```

## Data Flow

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Document Upload                                 │
│   POST /api/workspaces/:id/documents                                │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   DocumentTaskProcessor                              │
│   1. get_workspace_provider_lineage(workspace_id)                   │
│      → ProviderLineage {                                             │
│          extraction_provider: "openai",                              │
│          extraction_model: "gpt-4o-mini",                            │
│          embedding_provider: "openai",                               │
│          embedding_model: "text-embedding-3-small",                  │
│        }                                                             │
│   2. get_workspace_pipeline(workspace_id)                            │
│   3. pipeline.process() → result                                     │
│   4. Augment result.stats with provider lineage                      │
│   5. update_document_status_with_stats()                             │
└──────────────────────────────────┬──────────────────────────────────┘
                                   │
                                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     KV Storage (Document Metadata)                   │
│   {                                                                  │
│     "status": "completed",                                           │
│     "llm_provider": "openai",                                        │
│     "llm_model": "gpt-4o-mini",                                      │
│     "embedding_provider": "openai",                                  │
│     "embedding_model": "text-embedding-3-small",                     │
│     "embedding_dimensions": 1536,                                    │
│     "entity_count": 15,                                              │
│     "...": "..."                                                     │
│   }                                                                  │
└─────────────────────────────────────────────────────────────────────┘
```

## Test Results

All existing tests pass:

- e2e_workspace_provider_ingestion: 11 tests ✅
- e2e_workspace_provider_rebuild: 6 tests ✅
- e2e_postgres_provider_switching: 8 tests ✅

## Next Steps

- OODA 200: Add E2E test for lineage tracking
- OODA 201-210: Continue verification
