# Act

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Implementation commit: `1954ffa0` (`OODA-01: harden rate limit response handling`)

## Implemented Changes

1. Added conservative retry-delay rounding in [limiter.rs](../../../edgequake/crates/edgequake-rate-limiter/src/limiter.rs#L96)
   - `TokenBucket::retry_after_seconds()` now rounds sub-second deficits up instead of flooring them to `0`.
   - `round_positive_retry_delay()` centralizes the invariant.

2. Added reset-timing state in [limiter.rs](../../../edgequake/crates/edgequake-rate-limiter/src/limiter.rs#L165)
   - `RateLimiter::get_state()` now returns `reset_after_seconds`.
   - `TokenBucket::seconds_until_full()` returns `0` when already full and otherwise reports rounded-up refill time.

3. Reworked HTTP response construction in [middleware.rs](../../../edgequake/crates/edgequake-rate-limiter/src/middleware.rs#L95)
   - Replaced manual JSON string building with typed `Json(RateLimitExceededBody)`.
   - Replaced direct `unwrap()` header insertion with `insert_u64_header()` and `insert_rate_limit_headers()`.
   - Successful responses now emit bucket-derived `X-RateLimit-Reset` values instead of a hardcoded placeholder.

4. Added edge-case tests in:
   - [limiter.rs](../../../edgequake/crates/edgequake-rate-limiter/src/limiter.rs#L293)
   - [middleware.rs](../../../edgequake/crates/edgequake-rate-limiter/src/middleware.rs#L246)

## Verification Evidence

- `cargo fmt --check` -> passed
- `cargo clippy -p edgequake-rate-limiter --all-targets -- -D warnings` -> passed
- `cargo test -p edgequake-rate-limiter` -> passed
  - 15 unit tests
  - 15 integration tests
- `cargo test --workspace --lib --quiet` -> passed
  - 1148 library tests across the workspace

## Result

The rate limiter is now stricter about backpressure semantics and safer in the overload path:

```text
before: blocked request -> retry_after floor -> possible 0s -> client retries too early
after : blocked request -> retry_after ceil  -> minimum 1s -> safer pacing

before: middleware header/body build -> unwrap() in request path
after : middleware header/body build -> helper-based insertion, no panic dependency
```
