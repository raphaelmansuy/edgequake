# Orient

Date: 2026-04-10
Mission file re-read: `mission/01-improve.md`
Code commit under analysis: `1954ffa0`

## First-Principles Analysis

The rate limiter exists to preserve availability under pressure. Any logic in that path must satisfy two properties:

1. It must not panic while constructing the very response used to protect the system.
2. It must guide callers toward less load, not more load.

The pre-fix implementation violated both:

- `unwrap()` in middleware assumes header and JSON serialization are infallible enough for request-path code. That is the wrong availability tradeoff.
- `Duration::as_secs()` floors sub-second waits to `0`, which weakens backpressure semantics exactly when the system is already rejecting requests.

## Options Considered

### Option A: Keep behavior and add comments

- Benefit: lowest code churn.
- Risk: leaves the panic surface and incorrect retry semantics intact.
- Decision: rejected.

### Option B: Patch only middleware `unwrap()` calls

- Benefit: removes one class of request-path failure.
- Risk: still emits `Retry-After: 0` for sub-second deficits because the root cause sits inside the limiter.
- Decision: rejected.

### Option C: Fix retry timing at the bucket level and centralize header emission in middleware

- Benefit: smallest change that corrects semantics at the source and improves DRY inside the middleware.
- Benefit: lets tests assert a stable contract around JSON body and headers.
- Risk: adds one public field to `RateLimitState` (`reset_after_seconds`), which is a small surface-area increase.
- Decision: accepted.

## Risk Assessment

- Low regression risk: changes are isolated to the rate limiter crate.
- High reliability value: request-path panic surface removed and backoff guidance corrected.
- Hidden test posture improved: behavior is now explicit for both successful and rate-limited responses.
