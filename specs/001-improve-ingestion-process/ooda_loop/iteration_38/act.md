# Iteration 38: Act

## Changes Implemented

### 1. PipelineConfig Extended (pipeline.rs)

**File**: `edgequake/crates/edgequake-pipeline/src/pipeline.rs`

Added:

```rust
/// Timeout per chunk extraction in seconds.
#[serde(default = "default_chunk_timeout")]
pub chunk_extraction_timeout_secs: u64,

/// Maximum retry attempts per chunk.
#[serde(default = "default_max_retries")]
pub chunk_max_retries: u32,

/// Initial retry delay in milliseconds.
#[serde(default = "default_initial_retry_delay")]
pub initial_retry_delay_ms: u64,
```

Defaults:

- `chunk_extraction_timeout_secs: 60`
- `chunk_max_retries: 3`
- `initial_retry_delay_ms: 1000`

### 2. Error Types Added (error.rs)

**File**: `edgequake/crates/edgequake-pipeline/src/error.rs`

Added three new error variants:

```rust
/// Extraction timeout error.
ExtractionTimeout {
    chunk_index: usize,
    timeout_secs: u64,
    message: String,
}

/// Retry limit exhausted.
RetryExhausted {
    chunk_index: usize,
    attempts: u32,
    message: String,
}

/// Circuit breaker open.
CircuitBreakerOpen {
    failures: u32,
    retry_after_secs: u64,
}
```

### 3. Exponential Backoff Implemented (worker.rs)

**File**: `edgequake/crates/edgequake-tasks/src/worker.rs`

Replaced fixed delay with exponential backoff:

```rust
/// Calculate exponential backoff delay for a given retry attempt.
fn calculate_backoff_delay(
    attempt: u32,
    initial_delay_ms: u64,
    max_delay_ms: u64,
    multiplier: f64,
) -> u64 {
    let delay = initial_delay_ms as f64 * multiplier.powi(attempt as i32);
    (delay as u64).min(max_delay_ms)
}
```

Updated WorkerPoolConfig:

- `initial_retry_delay_ms: 1000` (was: `retry_delay_secs: 5`)
- `max_retry_delay_ms: 60000` (new: cap at 60s)
- `backoff_multiplier: 2.0` (new: exponential growth)

### Files Modified

| File          | Lines Changed | Description                 |
| ------------- | ------------- | --------------------------- |
| `pipeline.rs` | +50           | Timeout/retry config fields |
| `error.rs`    | +45           | Three new error types       |
| `worker.rs`   | +40           | Exponential backoff         |

### Next Steps

- Iteration 39: Add timeout wrapper to extraction
- Iteration 40: Add unit tests for backoff
- Iteration 41: Verify builds pass

## Commit

Pending build verification before commit.
