import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  NetworkError,
  resolveServerRootUrl,
  serverRootClient,
} from "../client";

describe("serverRootClient (UI-DRY-003)", () => {
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

  it("resolveServerRootUrl prefixes backend base for root paths", () => {
    expect(resolveServerRootUrl("/health")).toBe(
      "http://backend.test:8080/health",
    );
  });

  it("resolveServerRootUrl returns relative path when no base configured", () => {
    delete process.env.EDGEQUAKE_API_URL;
    expect(resolveServerRootUrl("/ready")).toBe("/ready");
  });

  it("parses JSON on success", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify({ status: "healthy", version: "1" }),
    } as Response);

    const result = await serverRootClient<{ status: string; version: string }>(
      "/health",
    );

    expect(fetchMock).toHaveBeenCalledWith(
      "http://backend.test:8080/health",
      expect.objectContaining({ method: "GET" }),
    );
    expect(result.status).toBe("healthy");
  });

  it("throws NetworkError on fetch failure", async () => {
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new TypeError("failed"));

    await expect(serverRootClient("/health")).rejects.toBeInstanceOf(
      NetworkError,
    );
  });

  it("silent probes avoid console.error (Next.js dev overlay)", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {});

    vi.spyOn(globalThis, "fetch").mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "Service Unavailable",
      headers: new Headers(),
      json: async () => ({
        message: "starting",
        status: 503,
        code: "UNAVAILABLE",
      }),
    } as Response);

    await expect(
      serverRootClient("/health", { silent: true }),
    ).rejects.toMatchObject({ status: 503 });

    expect(errorSpy).not.toHaveBeenCalled();
    expect(debugSpy).toHaveBeenCalled();
  });
});
