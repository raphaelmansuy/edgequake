# OODA-12: Observe - Query with Multiple Providers

## Mission Criteria

> "Ensure query works with both Ollama and OpenAI LLM providers"
> "Ensure query works for document uploaded via the UI"

## Query API Test Results (Ollama)

### Test 1: Generic Query

```bash
curl -s http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What is RAG?", "mode": "hybrid"}'
```

**Response**: Correctly explains the context doesn't contain RAG info, but lists entities found (EDGEQUAKE, OLLAMA, RUST).

### Test 2: Document-Specific Query

```bash
curl -s http://localhost:8080/api/v1/query \
  -H "Content-Type: application/json" \
  -d '{"query": "What is EdgeQuake written in?", "mode": "hybrid"}'
```

**Response**: 

> EdgeQuake is written in the RUST programming language.

✅ Correctly retrieves information from ingested document.

## Document State

```
Documents: 5 total
- 1 completed (entity_count: 3)
- 4 processing (stuck - likely from earlier tests without Ollama running)

Completed Document:
- ID: 771aa051-fb52-4c21-891e-6809608b5708
- Title: test_1770538010.md
- Entities: EDGEQUAKE, RUST, OLLAMA
- LLM Model: gemma3:12b
- Embedding Model: embeddinggemma:latest
```

## Current Health Response

```json
{
  "llm_provider_name": "ollama",
  "components": {
    "llm_provider": true
  }
}
```

## Query Mode Support

The query API supports multiple modes:

| Mode | Description |
|------|-------------|
| `local` | Entity-only search |
| `global` | Document-wide search |
| `hybrid` | Combined entity + document search |
| `mix` | Weighted combination |
| `naive` | Basic vector search |

## OpenAI Query Path

OpenAI query follows the same code path as Ollama:

1. `QueryEngine.query()` receives request
2. Gets LLM provider from `state.llm_provider`
3. Builds context from entities/relationships
4. Calls `llm_provider.chat()` for answer generation

The only difference is which provider implements `LLMProvider` trait.

## Code Locations

### Query Handler
**File**: `edgequake/crates/edgequake-api/src/handlers/query.rs`

### Query Engine
**File**: `edgequake/crates/edgequake-query/src/engine.rs`

### LLMProvider Trait
**File**: `edgequake/crates/edgequake-llm/src/traits.rs:137`

```rust
pub trait LLMProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    async fn chat(&self, messages: &[ChatMessage], ...) -> Result<LLMResponse>;
}
```
