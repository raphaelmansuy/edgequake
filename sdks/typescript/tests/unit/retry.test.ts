/**
 * Tests for retry middleware — exercises exponential backoff, status-based retry,
 * AbortError bypass, and network error retry.
 */
import { describe, expect, it, vi } from "vitest";
import { createRetryMiddleware } from "../../src/transport/retry.js";
import type { RequestOptions } from "../../src/transport/types.js";

function makeRequest(overrides?: Partial<RequestOptions>): RequestOptions {
  return { method: "GET", path: "/test", ...overrides };
}

describe("createRetryMiddleware", () => {
  it("retries on 429 status code", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 2,
      retryDelay: 1, // 1ms for fast tests
      retryStatusCodes: [429, 503],
    });

    let callCount = 0;
    const next = vi.fn(async () => {
      callCount++;
      if (callCount < 3) {
        return new Response("{}", { status: 429 });
      }
      return new Response("{}", { status: 200 });
    });

    const response = await middleware(makeRequest(), next);
    expect(response.status).toBe(200);
    expect(next).toHaveBeenCalledTimes(3);
  });

  it("retries on 503 status code", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 1,
      retryDelay: 1,
      retryStatusCodes: [429, 503],
    });

    let callCount = 0;
    const next = vi.fn(async () => {
      callCount++;
      if (callCount === 1) {
        return new Response("{}", { status: 503 });
      }
      return new Response("{}", { status: 200 });
    });

    const response = await middleware(makeRequest(), next);
    expect(response.status).toBe(200);
    expect(next).toHaveBeenCalledTimes(2);
  });

  it("returns retryable response if all retries exhausted", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 1,
      retryDelay: 1,
      retryStatusCodes: [429],
    });

    const next = vi.fn(async () => new Response("{}", { status: 429 }));

    const response = await middleware(makeRequest(), next);
    // After maxRetries exhausted, returns the last response even if retryable
    expect(response.status).toBe(429);
    expect(next).toHaveBeenCalledTimes(2);
  });

  it("retries on network error", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 1,
      retryDelay: 1,
      retryStatusCodes: [429, 503],
    });

    let callCount = 0;
    const next = vi.fn(async () => {
      callCount++;
      if (callCount === 1) {
        throw new TypeError("fetch failed");
      }
      return new Response("{}", { status: 200 });
    });

    const response = await middleware(makeRequest(), next);
    expect(response.status).toBe(200);
    expect(next).toHaveBeenCalledTimes(2);
  });

  it("does NOT retry on AbortError", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 2,
      retryDelay: 1,
      retryStatusCodes: [429],
    });

    const next = vi.fn(async () => {
      throw new DOMException("The operation was aborted.", "AbortError");
    });

    await expect(middleware(makeRequest(), next)).rejects.toThrow(
      "The operation was aborted.",
    );
    expect(next).toHaveBeenCalledTimes(1);
  });

  it("throws last error after all network retries exhausted", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 1,
      retryDelay: 1,
      retryStatusCodes: [],
    });

    const next = vi.fn(async () => {
      throw new TypeError("fetch failed");
    });

    await expect(middleware(makeRequest(), next)).rejects.toThrow(
      "fetch failed",
    );
    expect(next).toHaveBeenCalledTimes(2);
  });

  it("does not retry non-retryable status codes", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 2,
      retryDelay: 1,
      retryStatusCodes: [429, 503],
    });

    const next = vi.fn(async () => new Response("{}", { status: 400 }));

    const response = await middleware(makeRequest(), next);
    expect(response.status).toBe(400);
    expect(next).toHaveBeenCalledTimes(1);
  });

  it("succeeds without retry when first call succeeds", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 3,
      retryDelay: 1,
      retryStatusCodes: [429],
    });

    const next = vi.fn(async () => new Response("{}", { status: 200 }));

    const response = await middleware(makeRequest(), next);
    expect(response.status).toBe(200);
    expect(next).toHaveBeenCalledTimes(1);
  });
});
