# Iteration 38: Orient

## Gap Analysis: Issue 8 - Timeout and Retry Handling

### Current vs Required

| Requirement         | Current State | Required State        | Gap    |
| ------------------- | ------------- | --------------------- | ------ |
| Chunk timeout       | ❌ None       | 60s configurable      | HIGH   |
| Retry limit         | ✅ 3 retries  | Configurable          | LOW    |
| Exponential backoff | ❌ Fixed 5s   | 2^n \* 1s             | MEDIUM |
| Circuit breaker     | ❌ None       | Open after 5 failures | LOW    |
| Error messages      | ❌ Generic    | "Timeout after 60s"   | MEDIUM |

### Root Cause Analysis

1. **No Timeout**: `extractor.extract()` has no timeout wrapper
   - LLM calls can hang indefinitely on network issues
   - No visibility into stuck operations
2. **Fixed Retry Delay**: `retry_delay_secs: 5` hardcoded
   - Doesn't adapt to failure patterns
   - Can hammer failing services

3. **No Circuit Breaker**: Each failure retried independently
   - Cascade failures when provider down
   - Wastes resources on known-bad requests

### Solution Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    TIMEOUT & RETRY FLOW                     │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────┐     ┌──────────┐     ┌─────────────────────┐  │
│  │ Request │────▶│ Timeout  │────▶│ Circuit Breaker?    │  │
│  └─────────┘     │ (60s)    │     │ Open → Fail Fast    │  │
│                  └──────────┘     │ Closed → Try        │  │
│                       │          └─────────────────────┘  │
│                       ▼                     │              │
│  ┌─────────────────────────────────────────┐│              │
│  │                SUCCESS?                  ││              │
│  │  Yes → Return result                     ││              │
│  │  No  → Retry with backoff               ││              │
│  └─────────────────────────────────────────┘│              │
│                       │                     │              │
│                       ▼                     │              │
│  ┌─────────────────────────────────────────┐│              │
│  │ RETRY LOGIC                              ││              │
│  │ Attempt 1: Wait 1s                       ││              │
│  │ Attempt 2: Wait 2s                       ││              │
│  │ Attempt 3: Wait 4s                       ││              │
│  │ > max_retries → RetryExhausted error    ││              │
│  └─────────────────────────────────────────┘│              │
│                                             │              │
└─────────────────────────────────────────────────────────────┘
```

### Implementation Priority

1. **P1**: Add timeout config to PipelineConfig ✅ DONE
2. **P1**: Add error types (Timeout, RetryExhausted, CircuitBreaker) ✅ DONE
3. **P1**: Implement exponential backoff in WorkerPool ✅ DONE
4. **P2**: Wrap extraction in tokio::time::timeout (next iteration)
5. **P3**: Add circuit breaker state machine (future work)

## Impact Assessment

- **Backend**: Pipeline config changes, worker pool changes
- **Frontend**: Error messages will include timeout context
- **Tests**: Need timeout simulation tests
