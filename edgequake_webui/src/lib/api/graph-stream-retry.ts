/**
 * SPEC-021 R3 — transient-congestion retry helper for SSE graph streaming.
 *
 * The backend graph materialization endpoint admits at most
 * `graph_materialize_concurrent` (default 4) concurrent materializations.
 * When that cap is hit it returns a transient-congestion error carrying
 * `reason: "transient_congestion"` and `retry_after_secs`. This helper
 * retries the stream with exponential backoff + jitter so concurrent
 * clients don't thunder-herd the slot, while never retrying non-transient
 * errors (DB failures, graph_too_large, etc.).
 *
 * Design notes:
 * - Single source of truth for the retry policy (DRY): every SSE graph
 *   consumer routes through here instead of open-coding backoff.
 * - Backoff: base * 2^attempt + jitter, capped at maxDelay, and never
 *   less than the server's `retry_after_secs` hint when present.
 * - Abort-aware: a cancelled retry loop exits immediately without surfacing
 *   an error to the caller.
 */

import type { GraphStreamEvent } from "@/lib/api/edgequake";

/** Reason code the backend emits for transient graph materialization congestion. */
export const TRANSIENT_CONGESTION_REASON = "transient_congestion";

/** Default retry tuning for graph stream transient congestion. */
export interface TransientRetryOptions {
  /** Maximum retry attempts (default 4). */
  maxRetries?: number;
  /** Base delay in ms for exponential backoff (default 500). */
  baseDelayMs?: number;
  /** Maximum delay between retries in ms (default 8000). */
  maxDelayMs?: number;
  /** Abort signal; when aborted the loop exits immediately. */
  signal?: AbortSignal;
}

/** Default retry attempts — absorbs StrictMode double-mount + tenant switch. */
const DEFAULT_MAX_RETRIES = 4;
const DEFAULT_BASE_DELAY_MS = 500;
const DEFAULT_MAX_DELAY_MS = 8000;

/**
 * Compute the next retry delay: exponential backoff with full jitter,
 * never below the server's retry-after hint.
 */
export function computeRetryDelay(
  attempt: number,
  baseDelayMs: number,
  maxDelayMs: number,
  serverHintSecs?: number,
): number {
  const exponential = Math.min(baseDelayMs * 2 ** attempt, maxDelayMs);
  const jitter = Math.floor(Math.random() * baseDelayMs);
  const hintMs = serverHintSecs ? serverHintSecs * 1000 : 0;
  return Math.max(Math.min(exponential, maxDelayMs) + jitter, hintMs);
}

/**
 * Detect whether a stream event is a transient-congestion error that the
 * client should retry. Returns the retry-after hint when present.
 */
export function isTransientCongestionError(
  event: GraphStreamEvent,
): { isTransient: true; retryAfterSecs?: number } | { isTransient: false } {
  if (event.type === "error" && event.reason === TRANSIENT_CONGESTION_REASON) {
    return { isTransient: true, retryAfterSecs: event.retry_after_secs };
  }
  return { isTransient: false };
}

/** Sleep that resolves early when the abort signal fires. */
export function sleepWithAbort(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal?.aborted) {
      resolve();
      return;
    }
    const timer = setTimeout(() => {
      signal?.removeEventListener("abort", onAbort);
      resolve();
    }, ms);
    const onAbort = () => {
      clearTimeout(timer);
      resolve();
    };
    signal?.addEventListener("abort", onAbort, { once: true });
  });
}
