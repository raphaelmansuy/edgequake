# OODA 196: Observe - Provider Lineage Requirements

**Date**: 2025-01-15
**Focus**: Understanding provider lineage tracking requirements

## Spec Requirements (from 032-ollama-lmstudio-provider.md)

### Requirement 3

> On Query --> Ensure I can chose the current LLM Provider -> Ensure it used, traced and stored in the generated message (as lineage information)

### Requirement 15

> For each assistant message, you must store the provider and model used to generate the message as lineage information in the database.

### Requirement 23 (CRITICAL)

> VERY IMPORTANT: Ensure when I upload a document to a workspace, the llm provider and model used for document ingestion is the one associated with the workspace.

## Current State

### Document Structure

[document.rs#L54-88](../../../../edgequake/crates/edgequake-core/src/types/document.rs#L54)

```rust
pub struct Document {
    pub id: String,
    pub content: String,
    pub status: DocumentStatus,
    pub metadata: Option<serde_json::Value>, // <-- Lineage can go here
}
```

### DocumentLineage Structure

[lineage.rs#L334-360](../../../../edgequake/crates/edgequake-pipeline/src/lineage.rs#L334)

```rust
pub struct DocumentLineage {
    pub document_id: String,
    pub job_id: String,
    pub chunks: Vec<ChunkLineage>,
    pub entities: HashMap<String, EntityLineage>,
    // NO provider info currently!
}
```

### Processing Flow

[processor.rs#L360-410](../../../../edgequake/crates/edgequake-api/src/processor.rs#L360)

```rust
// Gets workspace-specific pipeline
let pipeline = self.get_workspace_pipeline(workspace_id).await;
// Processes document
let result = pipeline.process(&document_id, &data.text).await;
// BUT: No tracking of which provider was actually used!
```

## Gap Analysis

| Feature                   | Current State | Required |
| ------------------------- | ------------- | -------- |
| Store extraction provider | ❌            | ✅       |
| Store extraction model    | ❌            | ✅       |
| Store embedding provider  | ❌            | ✅       |
| Store embedding model     | ❌            | ✅       |
| API returns lineage       | ❌            | ✅       |
| UI displays lineage       | ❌            | ✅       |

## Proposed Solution

### Option 1: Add to Document Metadata (Simple)

```rust
{
  "metadata": {
    "extraction_provider": "openai",
    "extraction_model": "gpt-4o-mini",
    "embedding_provider": "openai",
    "embedding_model": "text-embedding-3-small",
    "extracted_at": "2025-01-15T12:00:00Z"
  }
}
```

### Option 2: Extend DocumentLineage (Complete)

```rust
pub struct DocumentLineage {
    // ... existing fields ...
    pub extraction_provider: String,
    pub extraction_model: String,
    pub embedding_provider: String,
    pub embedding_model: String,
}
```

### Option 3: Both (Recommended)

Store in both places for different use cases:

- Metadata for quick access in document APIs
- Lineage for detailed traceability

## Next Step

OODA 197: Orient - Decide which solution to implement
