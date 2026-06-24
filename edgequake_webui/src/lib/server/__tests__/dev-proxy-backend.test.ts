import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("node:child_process", () => ({
  execSync: vi.fn(),
}));

import { execSync } from "node:child_process";
import {
  discoverBackendUrl,
  probeEdgequakeHealth,
  resolveDevProxyBackend,
} from "../dev-proxy-backend";

describe("resolveDevProxyBackend", () => {
  const originalEdgequakeApiUrl = process.env.EDGEQUAKE_API_URL;
  const originalNextPublicApiUrl = process.env.NEXT_PUBLIC_API_URL;

  afterEach(() => {
    vi.mocked(execSync).mockReset();
    if (originalEdgequakeApiUrl === undefined) {
      delete process.env.EDGEQUAKE_API_URL;
    } else {
      process.env.EDGEQUAKE_API_URL = originalEdgequakeApiUrl;
    }
    if (originalNextPublicApiUrl === undefined) {
      delete process.env.NEXT_PUBLIC_API_URL;
    } else {
      process.env.NEXT_PUBLIC_API_URL = originalNextPublicApiUrl;
    }
  });

  it("probeEdgequakeHealth accepts healthy EdgeQuake payload", () => {
    vi.mocked(execSync).mockReturnValueOnce(
      '{"status":"healthy","version":"0.12.11"}',
    );

    expect(probeEdgequakeHealth("http://127.0.0.1:8081")).toBe(true);
  });

  it("probeEdgequakeHealth rejects non-EdgeQuake responses", () => {
    vi.mocked(execSync).mockImplementationOnce(() => {
      throw new Error("curl failed");
    });

    expect(probeEdgequakeHealth("http://127.0.0.1:8080")).toBe(false);
  });

  it("resolveDevProxyBackend prefers validated env URL", () => {
    process.env.EDGEQUAKE_API_URL = "http://127.0.0.1:8081";
    vi.mocked(execSync).mockReturnValueOnce(
      '{"status":"healthy","version":"0.12.11"}',
    );

    expect(resolveDevProxyBackend()).toBe("http://127.0.0.1:8081");
  });

  it("resolveDevProxyBackend auto-discovers when env points at wrong service", () => {
    process.env.NEXT_PUBLIC_API_URL = "http://127.0.0.1:8080";
    vi.mocked(execSync)
      .mockImplementationOnce(() => {
        throw new Error("401");
      })
      .mockReturnValueOnce("8081")
      .mockReturnValueOnce('{"status":"healthy","version":"0.12.11"}');

    expect(resolveDevProxyBackend()).toBe("http://127.0.0.1:8081");
  });

  it("discoverBackendUrl uses shared port selector script", () => {
    vi.mocked(execSync).mockReturnValueOnce("8081");

    expect(discoverBackendUrl()).toBe("http://127.0.0.1:8081");
    const [command] = vi.mocked(execSync).mock.calls[0] ?? [];
    expect(String(command)).toContain("select_edgequake_port.py");
    expect(String(command)).toContain("backend 8080 20");
  });
});
