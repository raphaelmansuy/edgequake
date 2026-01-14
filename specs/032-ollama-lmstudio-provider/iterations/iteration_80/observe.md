# OODA 80 - Observe: Model Capability Tests

## Current State
- 47 E2E tests (all passing)
- Provider type validation added

## Gap Identified
Model capabilities are not being validated:
1. Streaming capability for LLM models
2. Function calling capability
3. Vision capability for multimodal models
4. Context length for models

## Data Collection

### Capability Structure
```json
{
  "capabilities": {
    "streaming": true,
    "function_calling": true,
    "vision": false
  }
}
```

## Next Action
Add model capability validation tests:
1. LLM models have streaming capability
2. Multimodal models have vision capability
3. Embedding models have dimension in cost
