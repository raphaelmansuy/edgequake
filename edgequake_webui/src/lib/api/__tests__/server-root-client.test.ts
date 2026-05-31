import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import {
  NetworkError,
  resolveServerRootUrl,
  serverRootClient,
} from "../client";

describe("serverRootClient (UI-DRY-003)", () => {
  beforeEach(() => {
    vi.stubEnv("EDGEQUAKE_API_URL", "http://backend.test:8080");
  });

  afterEach(() => {
    vi.unstubAllEnvs();
    vi.restoreAllMocks();
  });

  it("resolveServerRootUrl prefixes backend base for root paths", () => {
    expect(resolveServerRootUrl("/health")).toBe(
      "http://backend.test:8080/health",
    );
  });

  it("resolveServerRootUrl returns relative path when no base configured", () => {
    vi.stubEnv("EDGEQUAKE_API_URL", "");
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
});
