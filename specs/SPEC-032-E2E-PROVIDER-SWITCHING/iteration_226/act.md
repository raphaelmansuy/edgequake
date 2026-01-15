# OODA 226: OBSERVE + ACT - Provider Tracking in ProcessingStats

## Summary

Fixed provider tracking in ProcessingStats and added 9 tests to verify provider names are correctly tracked when processing documents through workspace-specific pipelines.

## Code Changes

### 1. EntityExtractor trait (extractor.rs)
Added `provider_name()` method to EntityExtractor trait:
```rust
fn provider_name(&self) -> &str {
    "unknown"
}
```

### 2. LLMExtractor implementation (extractor.rs)
Implemented `provider_name()` in LLMExtractor:
```rust
fn provider_name(&self) -> &str {
    self.llm_provider.name()
}
```

### 3. Pipeline process() (pipeline.rs)
Now sets `stats.llm_provider` and `stats.embedding_provider`:
```rust
// Step 2: Extract entities
stats.llm_model = Some(extractor.model_name().to_string());
stats.llm_provider = Some(extractor.provider_name().to_string());

// Step 3: Generate embeddings
stats.embedding_model = Some(provider.model().to_string());
stats.embedding_provider = Some(provider.name().to_string());
```

## Tests Added

**File**: `edgequake/crates/edgequake-api/tests/e2e_provider_tracking_stats.rs`

### Provider Tracking Tests (9 tests)

1. `test_llm_extractor_provider_name_mock` - Mock extractor returns "mock"
2. `test_llm_extractor_provider_name_ollama` - Ollama extractor returns "ollama"
3. `test_llm_extractor_provider_name_lmstudio` - LMStudio extractor returns "lmstudio"
4. `test_workspace_stores_provider_names` - Workspace stores provider names
5. `test_provider_switch_updates_names` - Switch updates provider names
6. `test_independent_provider_tracking` - Independent tracking per workspace
7. `test_embedding_provider_name_tracking` - Embedding providers return correct names
8. `test_llm_provider_name_tracking` - LLM providers return correct names
9. `test_create_workspace_pipeline_uses_provider_config` - Pipeline uses workspace config

## Key Verification Points

### Provider Name Tracking
- EntityExtractor.provider_name() returns correct provider (mock/ollama/lmstudio)
- ProcessingStats.llm_provider now populated during extraction
- ProcessingStats.embedding_provider now populated during embedding

### Bug Fix
- **BEFORE**: ProcessingStats.llm_provider and embedding_provider were always None
- **AFTER**: Both fields are populated with actual provider names

## Test Results

```
running 9 tests
test test_workspace_stores_provider_names ... ok
test test_provider_switch_updates_names ... ok
test test_independent_provider_tracking ... ok
test test_create_workspace_pipeline_uses_provider_config ... ok
test test_llm_provider_name_tracking ... ok
test test_llm_extractor_provider_name_ollama ... ok
test test_llm_extractor_provider_name_mock ... ok
test test_llm_extractor_provider_name_lmstudio ... ok
test test_embedding_provider_name_tracking ... ok

test result: ok. 9 passed; 0 failed; 0 ignored
```

## Test Counts

- edgequake-api: 844 tests passing (+9 from OODA 226)
- Session total new tests: 69
