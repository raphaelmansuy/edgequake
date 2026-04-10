# OODA-31 — Orient

## Analysis

### First Principles
1. Rate limiter correctness = API stability — must be thoroughly tested
2. Audit events are compliance artifacts — builder correctness prevents audit trail corruption
3. `round_positive_retry_delay` guarantees `Retry-After: 0` never happens — a subtle but critical invariant

### Approach
- Test `round_positive_retry_delay` edge cases (zero, sub-second, multi-second)
- Test `RateLimiter` public API: check_rate_limit_with_cost, get_state, reset
- Test `AuditEvent` builder chain + `with_error` dual-set behavior
- Test `AuditEventBuilder` full chain + build
- Test `AuditSeverity` ordering
