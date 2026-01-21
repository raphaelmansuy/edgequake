# OODA 203 - Act: E2E Rebuild Provider Switching Tests

## Action Taken

Created comprehensive E2E tests for rebuild operations with provider switching.

## Test File

`edgequake/crates/edgequake-api/tests/e2e_rebuild_provider_switching.rs`

## Tests Implemented (9 tests)

| #   | Test Name                                                    | Purpose                                       | Status |
| --- | ------------------------------------------------------------ | --------------------------------------------- | ------ |
| 1   | test_rebuild_embeddings_returns_updated_provider_config      | Verify rebuild-embeddings uses new config     | ✅     |
| 2   | test_rebuild_embeddings_requires_force_if_unchanged          | Verify force flag validation                  | ✅     |
| 3   | test_rebuild_knowledge_graph_returns_updated_provider_config | Verify rebuild-kg uses new config             | ✅     |
| 4   | test_rebuild_knowledge_graph_requires_force_if_unchanged     | Verify force flag validation                  | ✅     |
| 5   | test_rebuild_workspace_isolation                             | Verify workspace A rebuild doesn't affect B   | ✅     |
| 6   | test_pipeline_uses_updated_config_after_rebuild              | Verify pipeline recreated after config change | ✅     |
| 7   | test_rebuild_nonexistent_workspace_returns_404               | Verify 404 for missing workspace              | ✅     |
| 8   | test_rebuild_embeddings_response_fields                      | Verify all response fields present            | ✅     |
| 9   | test_rebuild_knowledge_graph_response_fields                 | Verify all response fields present            | ✅     |

## Test Results

```
running 9 tests
test test_rebuild_knowledge_graph_response_fields ... ok
test test_rebuild_knowledge_graph_requires_force_if_unchanged ... ok
test test_rebuild_nonexistent_workspace_returns_404 ... ok
test test_rebuild_embeddings_response_fields ... ok
test test_rebuild_embeddings_returns_updated_provider_config ... ok
test test_rebuild_workspace_isolation ... ok
test test_rebuild_knowledge_graph_returns_updated_provider_config ... ok
test test_rebuild_embeddings_requires_force_if_unchanged ... ok
test test_pipeline_uses_updated_config_after_rebuild ... ok

test result: ok. 9 passed; 0 failed; 0 ignored
```

## Verification

The tests confirm:

1. **Provider config is updated** - Workspace config is updated BEFORE documents are queued
2. **Response reflects new config** - API response shows new provider/model/dimension
3. **Workspace isolation works** - Rebuilding workspace A does not affect workspace B
4. **Pipeline is recreated** - New pipeline instance created with updated config
5. **Force flag validation** - Must use force=true if config unchanged

## Next Steps

- OODA 204-206: Add PostgreSQL-specific rebuild tests
- OODA 207-210: Add provider lineage verification during rebuild
