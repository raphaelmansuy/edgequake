/**
 * Timing helper for the handful of tests that assert "this stays fast".
 *
 * WHY this exists: five test files used a single `performance.now()` pair and compared the
 * result to a sub-millisecond or low-millisecond bound. Those tests run inside a parallel
 * Vitest worker on a shared machine, so one GC pause or one descheduled slice lands inside the
 * sample and the suite fails on a measurement nobody asked about. Observed in a full run:
 *
 *   should normalize statuses quickly   165.5ms against a 100ms bound, then 5 clean runs
 *   should format tokens quickly         63.7ms against a  50ms bound, then 5 clean runs
 *   spread operator is fast for small arrays  44.3ms against a 5ms bound
 *
 * None of those were regressions. The fix is not a looser bound — that hides a real slowdown —
 * it is a measurement that is not dominated by scheduler noise.
 *
 * The number returned is the FASTEST run, which is the honest cost of the work. A real
 * regression makes every run slow, so a genuine problem still fails the assertion.
 */

/** How many timed runs to take. The minimum is what is reported. */
export const TIMING_RUNS = 3;

/**
 * Run `fn` once to warm the JIT, then return the fastest of `TIMING_RUNS` timed runs, in ms.
 *
 * The warm-up matters: the first pass pays compilation of the code under test, which is not the
 * algorithm's cost and is the largest single source of variance.
 */
export function fastestRunMs(fn: () => void): number {
  fn();
  let best = Number.POSITIVE_INFINITY;
  for (let run = 0; run < TIMING_RUNS; run += 1) {
    const start = performance.now();
    fn();
    best = Math.min(best, performance.now() - start);
  }
  return best;
}
