/**
 * Retry middleware with exponential backoff.
 *
 * WHY: Transient failures (429, 503, network errors) are common in
 * distributed systems. Automatic retry with backoff prevents cascading
 * failures while respecting server rate limits.
 *
 * @module transport/retry
 */

import type { Middleware, RequestOptions } from "./types.js";

/** Retry configuration. */
export interface RetryConfig {
  maxRetries: number;
  retryDelay: number;
  retryStatusCodes: number[];
}

/**
 * Creates retry middleware with exponential backoff and jitter.
 *
 * WHY: Jitter prevents thundering herd when many clients retry simultaneously.
 * Formula: delay * 2^attempt + random(0, delay/2)
 */
export function createRetryMiddleware(config: RetryConfig): Middleware {
  return async (req: RequestOptions, next) => {
    let lastError: Error | undefined;

    for (let attempt = 0; attempt <= config.maxRetries; attempt++) {
      try {
        const response = await next(req);

        // WHY: Check if response status requires retry (429, 503)
        if (
          config.retryStatusCodes.includes(response.status) &&
          attempt < config.maxRetries
        ) {
          const delay = calculateDelay(config.retryDelay, attempt);
          await sleep(delay);
          continue;
        }

        return response;
      } catch (error) {
        lastError = error as Error;

        // WHY: Don't retry aborted requests
        if (error instanceof DOMException && error.name === "AbortError") {
          throw error;
        }

        if (attempt < config.maxRetries) {
          const delay = calculateDelay(config.retryDelay, attempt);
          await sleep(delay);
        }
      }
    }

    throw lastError;
  };
}

/**
 * Calculate retry delay with exponential backoff and jitter.
 *
 * WHY: Pure exponential backoff can cause synchronized retries.
 * Adding jitter (random 0–50% of base delay) spreads retries over time.
 */
function calculateDelay(baseDelay: number, attempt: number): number {
  const exponential = baseDelay * Math.pow(2, attempt);
  const jitter = Math.random() * (baseDelay / 2);
  return Math.min(exponential + jitter, 30_000); // Cap at 30 seconds
}

/** Promise-based sleep utility. */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
