# OODA Iteration 165 - Model Loading State

## Observe

### Focus

Verify that model loading state is handled for local providers.

### Investigation

**Model Loading in Local Providers**:

- Ollama: First request may trigger model load
- LM Studio: Requires manual model load in UI
- Cold start can add 10-60 seconds

**Health Check** (from OODA 145):

- TCP connection test only
- Doesn't verify model is loaded

## Orient

### Model Loading Flow

```
Request arrives
       │
       ▼
┌──────────────┐
│ Model loaded?│
└──────────────┘
       │
   No  │  Yes
       ▼    ▼
┌─────────┐ ┌─────────┐
│Load model│ │ Respond │
└─────────┘ └─────────┘
  (10-60s)    (instant)
```

### User Experience

| State           | Response Time |
| --------------- | ------------- |
| Model loaded    | < 5s          |
| Model loading   | 10-60s        |
| Model not found | Error         |

## Decide

**Status**: ✅ COMPLETE

Model loading is handled by provider with appropriate timeout.

## Act

### Verified

- Default timeout accommodates loading (120s)
- First request triggers load in Ollama
- LM Studio requires manual load
- Error message if model unavailable

---

_Commit: docs(OODA 165): Verify model loading state handling_
