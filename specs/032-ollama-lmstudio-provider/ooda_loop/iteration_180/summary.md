# OODA Iteration 180 - Model Switching During Session

## Observe

### Focus
Verify that users can switch models during an active session.

### Investigation

**Model Switching UI**:
- Query interface has provider/model selector
- Can change at any time
- Next query uses new model

**Session State**:
- Previous responses retain original model info
- New queries use updated selection
- No session restart required

## Orient

### Model Switching Flow

```
User in session (Model A)
         │
         ▼
Change to Model B in selector
         │
         ▼
Confirm change
         │
         ▼
Next query uses Model B
         │
         ▼
Lineage shows Model B
```

### Historical Consistency

| Query | Model Used | Lineage Display |
|-------|-----------|-----------------|
| Q1 | gpt-4o | gpt-4o |
| Q2 | gpt-4o | gpt-4o |
| *switch* | | |
| Q3 | llama3.2 | llama3.2 |
| Q4 | llama3.2 | llama3.2 |

## Decide

**Status**: ✅ COMPLETE

Model switching works seamlessly during sessions.

## Act

### Verified

- Selector allows mid-session changes
- Next query uses new model
- Previous responses unchanged
- Lineage correctly reflects each model

---
*Commit: docs(OODA 180): Verify model switching during session*
