# OODA 202 - Decide: E2E Rebuild Provider Switching Tests

## Decision

Add comprehensive E2E tests to verify provider switching during rebuild operations.

## Test Plan

### Test Suite: `e2e_rebuild_provider_switching.rs`

1. **test_rebuild_embeddings_uses_updated_provider**

   - Create workspace with mock-v1 providers
   - Simulate document ingestion
   - Update workspace to mock-v2 providers
   - Call rebuild_embeddings handler
   - Verify: response shows new provider config

2. **test_rebuild_knowledge_graph_uses_updated_provider**

   - Create workspace with mock-llm-v1 provider
   - Simulate document ingestion
   - Update workspace to mock-llm-v2 provider
   - Call rebuild_knowledge_graph handler
   - Verify: response shows new provider config

3. **test_provider_lineage_captured_on_rebuild**

   - Create workspace with mock providers
   - Process document (capture lineage)
   - Update workspace providers
   - Rebuild and reprocess
   - Verify: lineage shows new provider

4. **test_rebuild_with_dimension_change**

   - Create workspace with 768 dimension
   - Process document
   - Update to 1536 dimension
   - Call rebuild_embeddings
   - Verify: vectors cleared, new dimension applied

5. **test_rebuild_clears_workspace_scoped_data_only**
   - Create two workspaces
   - Add documents to both
   - Rebuild one workspace
   - Verify: other workspace unaffected

## Implementation Strategy

Use the existing handler functions directly rather than HTTP client to:

1. Simplify test setup
2. Avoid need for running server
3. Test the core logic

## Files to Create

- `edgequake/crates/edgequake-api/tests/e2e_rebuild_provider_switching.rs`

## Next Step

Implement the test file.
