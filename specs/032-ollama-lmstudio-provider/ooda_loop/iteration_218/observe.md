# OODA Iteration 218 - Observe

## Focus: Document Ingestion with Workspace Provider

### Objective

Analyze how document ingestion uses workspace-specific LLM and embedding providers.

### Document Upload Flow

From [document.rs](../../../../edgequake/crates/edgequake-api/src/handlers/document.rs):

1. Upload handler receives document
2. Extracts workspace_id from TenantContext (X-Workspace-Id header)
3. Calls processor to process document

### Key Question

When a document is uploaded to a workspace:
1. Which LLM provider is used for entity extraction?
2. Which embedding provider is used for chunk embedding?
3. How is this stored for lineage tracking?

### Processor Flow Analysis

From [processor.rs](../../../../edgequake/crates/edgequake-api/src/services/processor.rs):

```rust
// Get workspace-specific pipeline
let pipeline = get_workspace_pipeline(&state, workspace_id).await;

// Process document with workspace pipeline
let result = pipeline.ingest_with_embedding_provider(
    document_content,
    workspace_embedding_provider  // ← Workspace-specific!
).await;
```

### Provider Lineage

From `get_workspace_provider_lineage()`:

```rust
pub async fn get_workspace_provider_lineage(
    state: &AppState,
    workspace_id: &str,
) -> ProviderLineage {
    ProviderLineage {
        extraction_provider: workspace.llm_provider,
        extraction_model: workspace.llm_model,
        embedding_provider: workspace.embedding_provider,
        embedding_model: workspace.embedding_model,
        embedding_dimension: workspace.embedding_dimension,
    }
}
```

### Key Verification Points

1. **Document upload with workspace header** - Does it use workspace providers?
2. **Lineage stored in ProcessingStats** - Is provider info recorded?
3. **Rebuild with new provider** - Do rebuilt documents use new provider?

### Next Steps

Create E2E tests for:
1. Document upload stores workspace provider lineage
2. Rebuild operation uses updated workspace provider
3. Provider switch + rebuild uses new provider
