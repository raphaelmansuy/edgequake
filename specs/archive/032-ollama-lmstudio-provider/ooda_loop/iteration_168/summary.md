# OODA Iteration 168 - Token Counting and Usage Metrics

## Observe

### Focus

Verify that token counting and usage metrics are tracked.

### Investigation

**Token Tracking**:

- LLM responses include token counts
- Used for cost estimation
- Displayed in UI for transparency

**Usage Response** (from model response):

```json
{
  "usage": {
    "prompt_tokens": 150,
    "completion_tokens": 50,
    "total_tokens": 200
  }
}
```

## Orient

### Token Usage Flow

```
LLM Request
    │
    ▼
Provider API
    │
    ▼
Response with usage
    │
    ▼
Store in response
    │
    ▼
Display in UI (optional)
```

### Cost Calculation

From model cards:

```toml
[cost]
prompt_per_1m = 0.15
completion_per_1m = 0.60
```

Cost = (prompt_tokens × prompt_rate + completion_tokens × completion_rate) / 1,000,000

## Decide

**Status**: ✅ COMPLETE

Token counting is available through provider responses.

## Act

### Verified

- Token counts in LLM responses
- Cost rates in model cards
- Cost calculation possible
- Transparency for users

---

_Commit: docs(OODA 168): Verify token counting and metrics_
