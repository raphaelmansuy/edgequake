# Iteration 38: Decide

## Decisions Made

### D1: PipelineConfig Extended with Timeout/Retry Settings

Added three new configuration fields:

- `chunk_extraction_timeout_secs: u64` (default: 60)
- `chunk_max_retries: u32` (default: 3)
- `initial_retry_delay_ms: u64` (default: 1000)

**Rationale**: Configuration-driven timeouts allow operators to tune based on:

- LLM provider latency characteristics
- Document complexity (long chunks = longer processing)
- Rate limit behavior

### D2: New Error Types for Visibility

Added three structured error variants:

- `ExtractionTimeout { chunk_index, timeout_secs, message }`
- `RetryExhausted { chunk_index, attempts, message }`
- `CircuitBreakerOpen { failures, retry_after_secs }`

**Rationale**: Structured errors enable:

- Clear user-facing messages ("Timeout after 60s")
- Programmatic error handling (retry vs fail fast)
- Metrics/alerting on timeout patterns

### D3: Exponential Backoff in WorkerPool

Replaced fixed `retry_delay_secs: 5` with:

- `initial_retry_delay_ms: 1000` (1s base)
- `max_retry_delay_ms: 60000` (60s cap)
- `backoff_multiplier: 2.0` (exponential)

Formula: `min(initial * 2^attempt, max)`

**Rationale**: Exponential backoff prevents:

- Hammering failing services
- Cascade failures
- Rate limit exhaustion

## Deferred Decisions

### D4: Circuit Breaker Implementation (DEFERRED)

While error types are defined, full circuit breaker state machine deferred:

- Requires shared state across workers
- Needs threshold tuning
- Lower priority than basic timeout

### D5: Timeout Wrapper on Extract (NEXT ITERATION)

Need to wrap `extractor.extract(&chunk)` with `tokio::timeout`:

```rust
match tokio::time::timeout(
    Duration::from_secs(config.chunk_extraction_timeout_secs),
    extractor.extract(&chunk)
).await {
    Ok(Ok(result)) => result,
    Ok(Err(e)) => // extraction error
    Err(_) => // timeout error
}
```

## Action Items

1. ✅ Add timeout config to PipelineConfig
2. ✅ Add error types to error.rs
3. ✅ Implement exponential backoff in worker.rs
4. 🔲 Add timeout wrapper to extraction (iteration 39)
5. 🔲 Add unit tests for backoff calculation
6. 🔲 Verify builds pass
