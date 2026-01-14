# OODA 66 - Decide: Cost Information Validation

## Decision

Add test for model cost information:

- All models have `cost` object
- All cost properties (input_per_1k, output_per_1k, embedding_per_1k) exist
- All cost values are non-negative

## Why Non-Negative Only

Different providers have different pricing:

- OpenAI: Has actual costs
- Ollama: $0 (local)
- Mock: $0

Test validates structure, not specific values.
