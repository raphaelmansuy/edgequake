# Decision - Iteration 02

## Changes

1. Add `llm_model`, `embedding_model`, `embedding_dimension` to Chunk struct
2. Add `with_models()` builder method
3. Add tests for model tracking and serialization

## Expected Outcome

Each chunk carries model provenance for full traceability.
