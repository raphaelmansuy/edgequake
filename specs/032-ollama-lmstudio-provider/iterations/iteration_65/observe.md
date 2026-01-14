# OODA 65 - Observe: Model Capability Coverage

## Current State

### Models API Capabilities Response

Each model has a `capabilities` object with:

- `context_length`: Number (max tokens)
- `max_output_tokens`: Number
- `supports_vision`: Boolean
- `supports_function_calling`: Boolean
- `supports_json_mode`: Boolean
- `supports_streaming`: Boolean (tested in OODA 63)
- `supports_system_message`: Boolean
- `embedding_dimension`: Number (for embeddings)

### Current Test Coverage

| Capability                | Tested     |
| ------------------------- | ---------- |
| supports_streaming        | ✅ OODA 63 |
| embedding_dimension       | ✅ Partial |
| context_length            | ❌         |
| max_output_tokens         | ❌         |
| supports_vision           | ❌         |
| supports_function_calling | ❌         |
| supports_json_mode        | ❌         |
| supports_system_message   | ❌         |

### Test Expansion Opportunity

Add tests for:

1. LLM models have positive context_length
2. LLM models support function_calling (modern models)
3. Vision models are flagged correctly

This validates the model registry is returning complete data.
