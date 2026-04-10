# Observe

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Code commit under analysis: `1954ffa0`

## Territory Map

- Rust workspace root: `edgequake/`
- Target crate: `edgequake/crates/edgequake-rate-limiter`
- Request-path files inspected:
  - `edgequake/crates/edgequake-rate-limiter/src/limiter.rs`
  - `edgequake/crates/edgequake-rate-limiter/src/middleware.rs`

## Verified Findings

1. `check_rate_limit_with_cost()` computed blocked retry delay with `Duration::as_secs()`, which floors sub-second delays to `0`.
   - Verified in [limiter.rs](../../../edgequake/crates/edgequake-rate-limiter/src/limiter.rs) at the pre-fix logic now replaced by lines 150-162.
   - Operational risk: blocked clients can receive retry guidance that says "retry now", which promotes hot-loop retries instead of backoff.

2. The Axum middleware built response headers and JSON body with multiple `unwrap()` calls in request handling.
   - Verified in [middleware.rs](../../../edgequake/crates/edgequake-rate-limiter/src/middleware.rs) at the refactored areas now living in lines 95-137.
   - Operational risk: panics in HTTP response construction are unacceptable in middleware because they turn overload handling into availability loss.

3. Successful responses emitted `X-RateLimit-Reset: 60` as a fixed placeholder rather than a value derived from bucket state.
   - Verified in [middleware.rs](../../../edgequake/crates/edgequake-rate-limiter/src/middleware.rs) at the pre-fix TODO now replaced by lines 83-89 and 118-127.
   - Product risk: observability and client-side pacing headers were misleading.

## Existing Test Surface

- `cargo test -p edgequake-rate-limiter` already covered basic token bucket behavior, tenant isolation, cost-based accounting, and middleware allow/block cases.
- Missing edge-case coverage before this iteration:
  - blocked sub-second retry delay rounding
  - successful response header shape beyond HTTP 200
  - blocked response body/header contract

## Architecture Snapshot

```text
Request
  |
  v
rate_limit_middleware
  |
  +--> extract tenant/workspace key
  |
  +--> RateLimiter::check_rate_limit()
          |
          v
      TokenBucket
          |
          +--> allow -> downstream handler + rate-limit headers
          |
          +--> deny  -> 429 JSON + Retry-After
```
