# OODA 68 - Decide: Default Configuration Validation

## Decision

Add test for default model configuration validity:

- Default LLM provider exists and is enabled
- Default LLM model exists in that provider (can be "llm" or "multimodal" type)
- Default embedding provider exists and is enabled
- Default embedding model exists and is "embedding" type
- Default embedding dimension is positive (if provided)

## Adjustments Made

1. Changed model type check from `== "llm"` to `in ["llm", "multimodal"]`
2. Made `default_embedding_dimension` optional (API doesn't always return it)
