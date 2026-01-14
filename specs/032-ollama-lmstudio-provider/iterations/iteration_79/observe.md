# OODA 79 - Observe: Provider Type Validation Tests

## Current State
- 43 E2E tests (all passing)
- All 8 focus areas + hardening tests covered

## Gap Identified
Need to validate provider type behavior:
1. LLM providers have LLM models
2. Embedding providers have embedding models  
3. Multimodal providers are correctly typed
4. Provider priority ordering is respected

## Data Collection

### Provider Types from API
- openai: llm, embedding
- ollama: llm, embedding, multimodal
- lmstudio: llm, embedding, multimodal
- anthropic: llm
- mock: llm, embedding

### Model Types
- llm: text generation
- embedding: vector embeddings
- multimodal: vision + text

## Next Action
Add provider type validation tests:
1. Verify each provider has expected model types
2. Verify provider priority sorting
3. Verify deprecated models are marked
