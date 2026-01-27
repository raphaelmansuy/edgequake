# OODA-16 Observe: Ollama E2E Testing

## Mission Requirement

From specs/033-study-delete-document/003-study-document.md:

> "Ensure e2e test are conducted also with ollama provider with real llm such as gemma3:latest and gemma:latest for embeddings. Ensure the create/update/delete and query document process is fully reliable with ollama provider. Ensure all mode of query works as expected: LLM-only, embedding-only, hybrid."

## Current Provider Architecture

### Provider Factory Location

File: `edgequake/crates/edgequake-llm/src/lib.rs`

The current factory pattern:

```
OPENAI_API_KEY set? → OpenAI provider
otherwise → Mock provider
```

### Ollama Support Status

1. **OpenAI-compatible API**: Ollama supports OpenAI API format at http://localhost:11434/v1/
2. **Environment Variables**:
   - `OLLAMA_HOST` - Ollama API endpoint (default: http://localhost:11434)
   - `OLLAMA_MODEL` - Model name (e.g., gemma3:latest)
   - `OLLAMA_EMBEDDING_MODEL` - Embedding model (e.g., nomic-embed-text)

### Current LLM Provider Implementation

```
edgequake/crates/edgequake-llm/src/
├── lib.rs                 # Provider factory
├── openai/
│   └── mod.rs            # OpenAIProvider
├── mock/
│   └── mod.rs            # MockProvider (testing)
└── traits.rs             # LLMProvider, EmbeddingProvider traits
```

## Ollama Integration Options

### Option A: Use OpenAI-compatible mode

Configure OpenAI provider with Ollama endpoint:

```env
OPENAI_API_KEY=ollama  # Dummy key
OPENAI_API_BASE=http://localhost:11434/v1
OPENAI_MODEL=gemma3:latest
```

### Option B: Create dedicated OllamaProvider

More explicit configuration and error handling specific to Ollama.

## Test Infrastructure Analysis

### Current E2E Tests

- `edgequake/crates/edgequake-api/tests/e2e_document_deletion.rs` - 25 tests (memory)
- Uses `AppState::test_state()` which creates mock provider

### What's Needed for Ollama E2E

1. Check if Ollama is running before tests
2. Skip tests if Ollama unavailable (CI compatibility)
3. Use real models (gemma3:latest, nomic-embed-text)
4. Verify full document lifecycle: add → query → delete
5. Verify all query modes: LLM-only, embedding-only, hybrid

## Ollama Model Requirements

### For LLM (Chat/Completion)

- `gemma3:latest` - Good balance of quality and speed
- `llama3.2:latest` - Alternative

### For Embeddings

- `nomic-embed-text` - Recommended, 768 dimensions
- `mxbai-embed-large` - Higher quality, slower

## Next Steps

1. Check if OpenAI provider supports base URL override
2. Create Ollama-specific test file
3. Add #[ignore] attribute for CI (run manually or with flag)
4. Implement full CRUD + query E2E tests
