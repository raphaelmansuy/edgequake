/**
 * Unit tests for the config module.
 *
 * @module tests/config.test
 */

import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { resolveConfig } from "../../src/config.js";

describe("resolveConfig", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    // Clear EdgeQuake env vars
    delete process.env.EDGEQUAKE_BASE_URL;
    delete process.env.EDGEQUAKE_API_KEY;
    delete process.env.EDGEQUAKE_TENANT_ID;
    delete process.env.EDGEQUAKE_USER_ID;
    delete process.env.EDGEQUAKE_WORKSPACE_ID;
  });

  afterEach(() => {
    // Restore
    Object.assign(process.env, originalEnv);
  });

  it("uses defaults when no config or env vars provided", () => {
    const config = resolveConfig();
    expect(config.baseUrl).toBe("http://localhost:8080");
    expect(config.apiKey).toBe("");
    expect(config.accessToken).toBe("");
    expect(config.tenantId).toBe("");
    expect(config.userId).toBe("");
    expect(config.workspaceId).toBe("");
    expect(config.timeout).toBe(30_000);
    expect(config.maxRetries).toBe(3);
    expect(config.fetchFn).toBe(globalThis.fetch);
  });

  it("uses explicit config over defaults", () => {
    const config = resolveConfig({
      baseUrl: "https://api.example.com",
      apiKey: "my-key",
      tenantId: "tenant-1",
      userId: "user-1",
      workspaceId: "ws-1",
      timeout: 5000,
      maxRetries: 5,
    });

    expect(config.baseUrl).toBe("https://api.example.com");
    expect(config.apiKey).toBe("my-key");
    expect(config.tenantId).toBe("tenant-1");
    expect(config.userId).toBe("user-1");
    expect(config.workspaceId).toBe("ws-1");
    expect(config.timeout).toBe(5000);
    expect(config.maxRetries).toBe(5);
  });

  it("reads from environment variables as fallback", () => {
    process.env.EDGEQUAKE_BASE_URL = "https://env.example.com";
    process.env.EDGEQUAKE_API_KEY = "env-key";
    process.env.EDGEQUAKE_TENANT_ID = "env-tenant";
    process.env.EDGEQUAKE_USER_ID = "env-user";
    process.env.EDGEQUAKE_WORKSPACE_ID = "env-workspace";

    const config = resolveConfig();
    expect(config.baseUrl).toBe("https://env.example.com");
    expect(config.apiKey).toBe("env-key");
    expect(config.tenantId).toBe("env-tenant");
    expect(config.userId).toBe("env-user");
    expect(config.workspaceId).toBe("env-workspace");
  });

  it("explicit config overrides env vars", () => {
    process.env.EDGEQUAKE_BASE_URL = "https://env.example.com";

    const config = resolveConfig({ baseUrl: "https://explicit.example.com" });
    expect(config.baseUrl).toBe("https://explicit.example.com");
  });

  it("accepts credentials", () => {
    const config = resolveConfig({
      credentials: { username: "admin", password: "secret" },
    });
    expect(config.credentials).toEqual({
      username: "admin",
      password: "secret",
    });
  });

  it("accepts custom fetch function", () => {
    const customFetch = (() => {}) as unknown as typeof fetch;
    const config = resolveConfig({ fetch: customFetch });
    expect(config.fetchFn).toBe(customFetch);
  });
});
