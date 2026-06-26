/**
 * SPEC-021 stabilization + P-G13 — backend readiness probes.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

function liveOk() {
  return {
    ok: () => true,
    status: () => 200,
    text: async () => "OK",
    json: async () => ({}),
  };
}

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

  it("probeBackendReadiness returns ready when /live and /health are healthy", async () => {
    const request = {
      get: vi.fn().mockImplementation((url: string) => {
        if (url.endsWith("/live")) return Promise.resolve(liveOk());
        return Promise.resolve({
          ok: () => true,
          status: () => 200,
          text: async () => "",
          json: async () => ({ status: "healthy" }),
        });
      }),
    };
    const { probeBackendReadiness, _resetBackendReadinessCache } =
      await import("../backend-readiness");
    _resetBackendReadinessCache();

    const state = await probeBackendReadiness(request);
    expect(state).toBe("ready");
    expect(request.get).toHaveBeenCalledWith("http://backend.test:8080/live");
    expect(request.get).toHaveBeenCalledWith("http://backend.test:8080/health");
  });

  it("isBackendReady returns true when /health reports degraded (busy, not down)", async () => {
    const request = {
      get: vi.fn().mockImplementation((url: string) => {
        if (url.endsWith("/live")) return Promise.resolve(liveOk());
        return Promise.resolve({
          ok: () => true,
          status: () => 200,
          text: async () => "",
          json: async () => ({ status: "degraded" }),
        });
      }),
    };
    const { isBackendReady, _resetBackendReadinessCache } =
      await import("../backend-readiness");
    _resetBackendReadinessCache();

    const ready = await isBackendReady(request);
    expect(ready).toBe(true);
  });

  it("probeBackendReadiness returns degraded when /live ok but /health throws", async () => {
    const request = {
      get: vi.fn().mockImplementation((url: string) => {
        if (url.endsWith("/live")) return Promise.resolve(liveOk());
        return Promise.reject(new TypeError("timeout"));
      }),
    };
    const { probeBackendReadiness, _resetBackendReadinessCache } =
      await import("../backend-readiness");
    _resetBackendReadinessCache();

    const state = await probeBackendReadiness(request);
    expect(state).toBe("degraded");
  });

  it("isBackendReady returns false when /live is unreachable after retries", async () => {
    const request = {
      get: vi.fn().mockRejectedValue(new TypeError("connect ECONNREFUSED")),
    };
    const { isBackendReady, _resetBackendReadinessCache } =
      await import("../backend-readiness");
    _resetBackendReadinessCache();

    const ready = await isBackendReady(request);
    expect(ready).toBe(false);
    expect(request.get).toHaveBeenCalledTimes(3);
  });

  it("isBackendReady returns true on /health 401/403 when /live ok", async () => {
    const request = {
      get: vi.fn().mockImplementation((url: string) => {
        if (url.endsWith("/live")) return Promise.resolve(liveOk());
        return Promise.resolve({
          ok: () => false,
          status: () => 401,
          text: async () => "Unauthorized",
          json: async () => ({ message: "Unauthorized" }),
        });
      }),
    };
    const { isBackendReady, _resetBackendReadinessCache } =
      await import("../backend-readiness");
    _resetBackendReadinessCache();

    const ready = await isBackendReady(request);
    expect(ready).toBe(true);
  });

  it("caches successful readiness within the cache window", async () => {
    const request = {
      get: vi.fn().mockImplementation((url: string) => {
        if (url.endsWith("/live")) return Promise.resolve(liveOk());
        return Promise.resolve({
          ok: () => true,
          status: () => 200,
          text: async () => "",
          json: async () => ({ status: "healthy" }),
        });
      }),
    };
    const { probeBackendReadiness, _resetBackendReadinessCache } =
      await import("../backend-readiness");
    _resetBackendReadinessCache();

    await probeBackendReadiness(request);
    await probeBackendReadiness(request);
    await probeBackendReadiness(request);

    expect(request.get).toHaveBeenCalledTimes(2);
  });
});
