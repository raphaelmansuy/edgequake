# Decide

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Code commit under analysis: `1954ffa0`

## Prioritized Actions

1. Correct retry-delay rounding in the token bucket so blocked requests never receive `0` seconds of retry guidance.
2. Expose computed reset timing from bucket state so middleware can emit truthful `X-RateLimit-Reset` values.
3. Replace ad hoc header and JSON response construction with small helpers that fail closed by skipping invalid headers instead of panicking.
4. Add tests for:
   - sub-second retry rounding
   - success-path header contract
   - 429 body and header contract
5. Verify with:
   - `cargo fmt --check`
   - `cargo clippy -p edgequake-rate-limiter --all-targets -- -D warnings`
   - `cargo test -p edgequake-rate-limiter`
   - `cargo test --workspace --lib --quiet`

## Why This Slice First

This iteration targets a narrow but high-signal request path:

- every API call can traverse it
- failures here degrade the entire service, not one feature
- the defect is measurable with deterministic tests

That makes it a better first mission slice than a broad speculative refactor.
