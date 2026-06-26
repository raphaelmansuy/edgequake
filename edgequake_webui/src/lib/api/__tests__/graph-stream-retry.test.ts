/**
 * SPEC-021 R3 — graph stream transient-congestion retry helper tests.
 */
import { describe, expect, it } from "vitest";

import type { GraphStreamEvent } from "@/lib/api/edgequake";
import {
  computeRetryDelay,
  isTransientCongestionError,
  sleepWithAbort,
  TRANSIENT_CONGESTION_REASON,
} from "@/lib/api/graph-stream-retry";

describe("computeRetryDelay", () => {
  it("grows exponentially and is capped by maxDelayMs", () => {
    const base = 500;
    const max = 8000;
    // attempt 0: min(500, 8000) = 500 (+ jitter 0..499) → at least 500
    expect(computeRetryDelay(0, base, max)).toBeGreaterThanOrEqual(500);
    // attempt 5: 500*32=16000 capped to 8000 (+ jitter) → ≤ 8000 + 499
    expect(computeRetryDelay(5, base, max)).toBeLessThanOrEqual(max + base);
  });

  it("respects the server retry-after hint as a floor", () => {
    const delay = computeRetryDelay(0, 500, 8000, 10 /* 10s hint */);
    // hint is 10000ms, so delay must be at least 10000 (exponential+jitter is smaller)
    expect(delay).toBeGreaterThanOrEqual(10000);
  });

  it("never returns a negative or NaN delay", () => {
    for (let attempt = 0; attempt < 10; attempt++) {
      const d = computeRetryDelay(attempt, 500, 8000);
      expect(Number.isFinite(d)).toBe(true);
      expect(d).toBeGreaterThanOrEqual(0);
    }
  });
});

describe("isTransientCongestionError", () => {
  it("returns isTransient:true for an error event with the transient reason", () => {
    const event: GraphStreamEvent = {
      type: "error",
      message: "Graph materialization capacity reached",
      reason: TRANSIENT_CONGESTION_REASON,
      retry_after_secs: 5,
    };
    const result = isTransientCongestionError(event);
    expect(result.isTransient).toBe(true);
    if (result.isTransient) {
      expect(result.retryAfterSecs).toBe(5);
    }
  });

  it("returns isTransient:false for a non-transient error (no reason)", () => {
    const event: GraphStreamEvent = {
      type: "error",
      message: "Failed to fetch edges: boom",
    };
    expect(isTransientCongestionError(event).isTransient).toBe(false);
  });

  it("returns isTransient:false for a non-error event", () => {
    const event: GraphStreamEvent = {
      type: "metadata",
      total_nodes: 10,
      total_edges: 20,
      nodes_to_stream: 5,
      edges_to_stream: 10,
    };
    expect(isTransientCongestionError(event).isTransient).toBe(false);
  });

  it("returns isTransient:false for an error with a different reason code", () => {
    const event: GraphStreamEvent = {
      type: "error",
      message: "Graph too large",
      reason: "graph_too_large",
    };
    expect(isTransientCongestionError(event).isTransient).toBe(false);
  });
});

describe("sleepWithAbort", () => {
  it("resolves after the requested duration when not aborted", async () => {
    const start = Date.now();
    await sleepWithAbort(50);
    expect(Date.now() - start).toBeGreaterThanOrEqual(40);
  });

  it("resolves immediately when the signal is already aborted", async () => {
    const controller = new AbortController();
    controller.abort();
    const start = Date.now();
    await sleepWithAbort(1000, controller.signal);
    expect(Date.now() - start).toBeLessThan(100);
  });

  it("resolves early when the signal fires mid-sleep", async () => {
    const controller = new AbortController();
    const start = Date.now();
    setTimeout(() => controller.abort(), 30);
    await sleepWithAbort(5000, controller.signal);
    expect(Date.now() - start).toBeLessThan(500);
  });
});
