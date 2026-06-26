/**
 * SPEC-021 stabilization — backend readiness + silent network logging.
 *
 * Covers the edge case: frontend starts before the backend is ready.
 * The dashboard must not crash (no console.error → no Next.js dev overlay),
 * and `isBackendReady` must report false until /health returns healthy.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

describe("SPEC-021 backend readiness", () => {
  const originalApiUrl = process.env.EDGEQUAKE_API_URL;

  beforeEach(() => {
    process.env.EDGEQUAKE_API_URL = "http://backend.test:8080";
  });

  afterEach(() => {
    if (originalApiUrl === undefined) {
      delete process.env.EDGEQUAKE_API_URL;
    } else {
      process.env.EDGEQUAKE_API_URL = originalApiUrl;
    }
    vi.restoreAllMocks();
  });

  it("isBackendReady returns true when /health reports healthy", async () => {
    const request = {
      get: vi.fn().mockResolvedValue({
        ok: () => true,
        status: () => 200,
        json: async () => ({ status: "healthy" }),
      }),
    };
    const { isBackendReady, _resetBackendReadinessCache } = await import("../client");
    _resetBackendReadinessCache();

    const ready = await isBackendReady(request);
    expect(ready).toBe(true);
    expect(request.get).toHaveBeenCalledWith("http://backend.test:8080/health");
  });

  it("isBackendReady returns false when /health reports starting", async () => {
    const request = {
      get: vi.fn().mockResolvedValue({
        ok: () => true,
        status: () => 200,
        json: async () => ({ status: "starting" }),
      }),
    };
    const { isBackendReady, _resetBackendReadinessCache } = await import("../client");
    _resetBackendReadinessCache();

    const ready = await isBackendReady(request);
    expect(ready).toBe(false);
  });

  it("isBackendReady returns false when the backend is unreachable", async () => {
    const request = {
      get: vi.fn().mockRejectedValue(new TypeError("connect ECONNREFUSED")),
    };
    const { isBackendReady, _resetBackendReadinessCache } = await import("../client");
    _resetBackendReadinessCache();

    const ready = await isBackendReady(request);
    expect(ready).toBe(false);
  });

  it("isBackendReady returns false when /health responds non-2xx (non-auth)", async () => {
    const request = {
      get: vi.fn().mockResolvedValue({
        ok: () => false,
        status: () => 500,
        json: async () => ({}),
      }),
    };
    const { isBackendReady, _resetBackendReadinessCache } = await import("../client");
    _resetBackendReadinessCache();

    const ready = await isBackendReady(request);
    expect(ready).toBe(false);
  });

  it("isBackendReady returns true on 401/403 (backend up, auth-gated health)", async () => {
    // WHY: some deployments put /health behind auth. A 401 means the backend
    // is reachable and responding — the banner must not fire.
    const request = {
      get: vi.fn().mockResolvedValue({
        ok: () => false,
        status: () => 401,
        json: async () => ({ message: "Unauthorized" }),
      }),
    };
    const { isBackendReady, _resetBackendReadinessCache } = await import("../client");
    _resetBackendReadinessCache();

    const ready = await isBackendReady(request);
    expect(ready).toBe(true);
  });

  it("isBackendReady caches the result within the cache window", async () => {
    const request = {
      get: vi.fn().mockResolvedValue({
        ok: () => true,
        status: () => 200,
        json: async () => ({ status: "healthy" }),
      }),
    };
    const { isBackendReady, _resetBackendReadinessCache } = await import("../client");
    _resetBackendReadinessCache();

    await isBackendReady(request);
    await isBackendReady(request);
    await isBackendReady(request);

    // Three calls, one network request (cached afterwards)
    expect(request.get).toHaveBeenCalledTimes(1);
  });
});
