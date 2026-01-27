# OODA Iteration 213 - Decide

## Decision: Create E2E Test for Document Processing Lineage

### Problem Statement

Need to verify that when a workspace is configured with specific providers, document processing:

1. Uses the configured providers (not defaults)
2. Stores the correct provider info in document metadata (lineage)
3. Retrieves the correct lineage when querying document stats

### Existing Code Analysis

The [`processor.rs`](../../../../edgequake/crates/edgequake-api/src/processor.rs) already:

1. **Gets workspace lineage** (line 449):

   ```rust
   let provider_lineage = self.get_workspace_provider_lineage(workspace_id).await;
   ```

2. **Populates stats with lineage** (lines 714-720):

   ```rust
   stats_with_lineage.llm_provider = Some(provider_lineage.extraction_provider.clone());
   stats_with_lineage.embedding_provider = Some(provider_lineage.embedding_provider.clone());
   ```

3. **Stores stats in KV storage** (lines 821-830):
   ```rust
   if let Some(ref llm_provider) = stats.llm_provider {
       updated.insert("llm_provider".to_string(), json!(llm_provider));
   }
   ```

### Test Design

Create a new E2E test that:

1. **Setup**: Create workspace with specific provider (e.g., "mock")
2. **Action**: Upload/process a document in that workspace
3. **Verify**: Query the document metadata to confirm provider lineage matches workspace config

### Test Implementation Plan

```rust
#[tokio::test]
async fn test_document_processing_stores_workspace_provider_lineage() {
    // 1. Create workspace with mock provider
    let workspace = create_workspace(provider: "mock", model: "mock-model");

    // 2. Upload a document to the workspace
    let doc = upload_document(&workspace, "Test content");

    // 3. Wait for processing to complete
    wait_for_processing(&doc);

    // 4. Get document metadata
    let metadata = get_document_metadata(&doc);

    // 5. Verify lineage matches workspace config
    assert_eq!(metadata["llm_provider"], "mock");
    assert_eq!(metadata["embedding_provider"], "mock");
}
```

### Edge Cases to Test

1. **Provider switching**: Change workspace provider, rebuild, verify new lineage
2. **Workspace isolation**: Two workspaces with different providers, verify isolated lineage
3. **Fallback behavior**: When provider creation fails, verify lineage shows fallback

### Location

Create test in: [`e2e_document_lineage.rs`](../../../../edgequake/crates/edgequake-api/tests/e2e_document_lineage.rs)

### Dependencies

- Need to understand how to trigger document processing synchronously in tests
- May need to use task queue or direct processor invocation
