# OODA 66 - Observe: Model Cost Information

## Current State

### Model Cost API Response

Each model includes a `cost` object:

```json
{
  "cost": {
    "input_per_1k": 0.0025,
    "output_per_1k": 0.01,
    "embedding_per_1k": 0.0,
    "image_per_unit": 0.0
  }
}
```

### Test Coverage Gap

No tests currently verify cost information:

- LLM models should have input/output costs
- Embedding models should have embedding costs
- Cost values should be non-negative

### UI Usage

The Costs page displays model pricing. Without cost data validation:

- UI might show $0.00 for all models
- Cost calculations could be wrong

## Opportunity

Add test to verify models have cost information present.
