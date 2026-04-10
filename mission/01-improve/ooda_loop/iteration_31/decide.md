# OODA-31 — Decide

## Plan

1. Add edge case tests to `limiter.rs`:
   - `round_positive_retry_delay` — zero, sub-second, exact seconds, large values
   - `RateLimiter` — cost-based check, get_state, reset
2. Add edge case tests to `event.rs`:
   - `AuditEvent::new` all defaults, with_error dual-set, severity builder, builder chain
   - `AuditEventBuilder` full chain + build
   - `AuditSeverity` ordering (derive Ord)
3. Run tests, commit as OODA-31

**Expected: ~15 new tests**
