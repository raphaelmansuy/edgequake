# OODA Loop Iteration 287: Test Speed Optimization

## Observe

### Slow Sleep Locations Found

| File                              | Line | Duration | Purpose                  |
| --------------------------------- | ---- | -------- | ------------------------ |
| edgequake-llm/rate_limiter.rs     | 489  | 2s       | Token bucket refill test |
| edgequake-rate-limiter/limiter.rs | 232  | 600ms    | Rate limit window test   |
| edgequake-query/test_fixtures.rs  | 390  | 500ms    | Unknown                  |
| edgequake-api/e2e_graph.rs        | 390  | 500ms    | Graph test wait          |
| edgequake-api/e2e_graph.rs        | 446  | 500ms    | Graph test wait          |
| edgequake-api/e2e_graph.rs        | 505  | 500ms    | Graph test wait          |
| edgequake-api/e2e_graph.rs        | 559  | 500ms    | Graph test wait          |

**Total Wasted Time**: ~4.6s just in explicit sleeps!

### Analysis

The 2s sleep in `rate_limiter.rs` is the biggest offender. It's testing token bucket refill behavior.

**Problem**: Real time waits in tests are anti-pattern:

1. Slow feedback loop
2. Flaky (time-dependent)
3. Wasteful of CI resources

## Orient

### First Principles: Why Do We Sleep?

1. **Token bucket refill**: Waiting for bucket to refill
2. **Rate limit window**: Waiting for window to reset
3. **Async operation**: Waiting for background task

### Solutions (in order of preference):

1. **Mock time**: Use `tokio::time::pause()` + `advance()`
2. **Instant refill**: Parameterize refill rate for tests
3. **No wait needed**: Redesign test to not need timing

## Decide

### Action Plan:

1. ✅ Identify all slow sleeps (DONE - 7 locations)
2. 🔲 Optimize rate_limiter.rs test (2s → instant)
3. 🔲 Optimize e2e_graph.rs tests (4x500ms → faster)
4. 🔲 Verify optimization doesn't break functionality
5. 🔲 Measure improvement

## Act

### Optimization Strategy for rate_limiter.rs:

The test waits 2s for token bucket to refill. We can use `tokio::time::pause()`
and `tokio::time::advance()` to simulate time passing instantly.

```rust
// BEFORE (slow):
tokio::time::sleep(Duration::from_secs(2)).await;

// AFTER (instant):
tokio::time::pause();
tokio::time::advance(Duration::from_secs(2)).await;
tokio::time::resume();
```

### Expected Improvement:

- 2s → 0s for rate_limiter test
- 4x500ms → 4x0s for e2e_graph tests (if we can mock time)
- Total: ~4.6s saved = **40% faster LLM test suite**

---

## Next Actions

1. Apply time mocking to rate_limiter.rs
2. Verify test still passes
3. Apply to other sleep locations where safe
