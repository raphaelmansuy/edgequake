# OODA-16 Decide: Ollama E2E Testing Implementation

## Decision

Create comprehensive E2E tests for document lifecycle with Ollama provider.

## Implementation Steps

### Step 1: Create Ollama Test Helper Module

Location: `edgequake/crates/edgequake-api/tests/ollama_test_helpers.rs`

Functions:

- `is_ollama_available() -> bool` - Check if Ollama is reachable
- `create_ollama_app_state() -> AppState` - Create state with real Ollama
- `upload_document_ollama(app, title, content) -> (StatusCode, Value)` - Upload with timeout

### Step 2: Create Ollama E2E Test File

Location: `edgequake/crates/edgequake-api/tests/e2e_ollama_integration.rs`

Tests (all marked `#[ignore]` for CI safety):

1. **test_ollama_availability_check**
   - Verify Ollama is reachable
   - Verify gemma3:latest is available
   - Verify nomic-embed-text is available

2. **test_ollama_document_upload_and_extraction**
   - Upload document about "Alice works at TechCorp with Bob"
   - Verify entities extracted (ALICE, BOB, TECHCORP)
   - Verify relationships created (WORKS_AT, WORKS_WITH)

3. **test_ollama_query_llm_only_mode**
   - Upload document
   - Query with mode=llm_only
   - Verify response uses LLM reasoning

4. **test_ollama_query_embedding_only_mode**
   - Upload document
   - Query with mode=embedding_only
   - Verify response uses vector similarity

5. **test_ollama_query_hybrid_mode**
   - Upload document
   - Query with mode=hybrid
   - Verify response combines graph + vector + LLM

6. **test_ollama_document_deletion_clears_entities**
   - Upload document, verify entities
   - Delete document
   - Verify all entities and relationships removed

## Acceptance Criteria

- [ ] All 6 tests pass when Ollama is running
- [ ] Tests skip gracefully when Ollama is not available
- [ ] Entity extraction produces meaningful entities (not mock)
- [ ] All query modes produce valid responses
- [ ] Deletion cascade works correctly

## Run Command

```bash
# Run Ollama E2E tests specifically
cargo test --package edgequake-api --test e2e_ollama_integration -- --ignored

# With verbose output
cargo test --package edgequake-api --test e2e_ollama_integration -- --ignored --nocapture
```
