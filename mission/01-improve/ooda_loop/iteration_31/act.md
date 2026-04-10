# OODA-31 — Act

## Changes Made

### 1. Rate Limiter tests: `limiter.rs` (+7 tests → 14 total)
- `test_round_positive_zero_duration` — ensures min 1s (no hot-loop)
- `test_round_positive_sub_second` — 100ms → ceil to 1
- `test_round_positive_exact_seconds` — 3s → 3
- `test_round_positive_fractional_seconds` — 2.1s → 3
- `test_get_state_absent_key` — returns None
- `test_get_state_after_check` — verifies token consumption reflected
- `test_reset_restores_capacity` — reset then re-check succeeds

### 2. Audit Event tests: `event.rs` (+8 tests → 11 total)
- `test_audit_event_new_defaults` — all 14 default fields verified
- `test_with_error_sets_result_to_failure` — dual-set behavior
- `test_severity_ordering` — Low < Medium < High < Critical
- `test_builder_error_sets_result` — AuditEventBuilder error path
- `test_builder_resource` — resource_type + resource_id
- `test_builder_category_and_duration` — category override + duration_ms
- `test_with_request_context` — ip, user_agent, request_id
- `test_with_duration` — duration builder

## Test Evidence

- **edgequake-rate-limiter**: 22 passed
- **edgequake-audit**: 13 passed
- **Workspace total**: 1352 passed, 0 failed
