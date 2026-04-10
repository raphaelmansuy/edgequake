# OODA-31 — Observe

## Target: Rate Limiter + Audit Event Builders

### Files Analyzed

1. **`crates/edgequake-rate-limiter/src/limiter.rs`** (~220 lines, 2 tests)
   - `round_positive_retry_delay()` — pure fn, rounds Duration to at-least-1 u64 seconds
   - `RateLimiter::check_rate_limit()` — synchronous, uses DashMap
   - `TokenBucket` — private struct with `refill()`, `try_consume()`, `time_until_available()`

2. **`crates/edgequake-audit/src/event.rs`** (~250 lines, 3 tests)
   - `AuditEvent::new()` — builder with defaults
   - `AuditEvent::with_*()` — 7 builder methods (workspace, user, severity, resource, request_context, metadata, error, duration)
   - `AuditEventBuilder` — separate builder pattern, untested
   - `AuditSeverity` — has `Ord` derive (Low < Medium < High < Critical)

### Test Gaps

| Function | Tests | Gap |
|----------|-------|-----|
| round_positive_retry_delay | 0 | Edge cases: zero Duration, sub-second, exact seconds |
| RateLimiter::check_rate_limit | 2 | Basic only; no cost-based, reset, get_state tests |
| AuditEvent::new defaults | 1 | Not all default fields verified |
| AuditEvent::with_error | 0 | Sets both error_message AND result=Failure |
| AuditEventBuilder | 0 | Entirely untested |
| AuditSeverity ordering | 0 | Low < Medium < High < Critical untested |
