/**
 * SPEC-020 / FIX-DEV-PROXY — runtime config dev vs prod API URL resolution.
 */
import { afterEach, describe, expect, it } from "bun:test";
import {
  getRuntimeApiBaseUrl,
  getRuntimeConfig,
  parseHealthPollIntervalMs,
} from "../runtime-config";

const ORIGINAL_ENV = { ...process.env };

afterEach(() => {
  process.env = { ...ORIGINAL_ENV };
  if (typeof window !== "undefined") {
    delete window.__EDGEQUAKE_RUNTIME_CONFIG__;
  }
});

describe("runtime-config", () => {
  it("uses relative /api/v1 in development (dev proxy)", () => {
    process.env.NODE_ENV = "development";
    delete process.env.EDGEQUAKE_API_URL;
    process.env.NEXT_PUBLIC_API_URL = "http://localhost:8080";
    expect(getRuntimeConfig().apiUrl).toBe("");
    expect(getRuntimeApiBaseUrl()).toBe("/api/v1");
  });

  it("uses relative /api/v1 in development even when EDGEQUAKE_API_URL is set", () => {
    process.env.NODE_ENV = "development";
    process.env.EDGEQUAKE_API_URL = "http://127.0.0.1:8080";
    process.env.NEXT_PUBLIC_API_URL = "http://localhost:8080";
    expect(getRuntimeConfig().apiUrl).toBe("");
    expect(getRuntimeApiBaseUrl()).toBe("/api/v1");
  });

  it("uses EDGEQUAKE_API_URL in production", () => {
    process.env.NODE_ENV = "production";
    process.env.EDGEQUAKE_API_URL = "http://api.example.com";
    delete process.env.NEXT_PUBLIC_API_URL;
    expect(getRuntimeConfig().apiUrl).toBe("http://api.example.com");
    expect(getRuntimeApiBaseUrl()).toBe("http://api.example.com/api/v1");
  });

  it("disables health poll by default", () => {
    delete process.env.EDGEQUAKE_HEALTH_POLL_MS;
    expect(getRuntimeConfig().healthPollIntervalMs).toBe(false);
  });

  it("enables health poll when EDGEQUAKE_HEALTH_POLL_MS is a positive integer", () => {
    process.env.EDGEQUAKE_HEALTH_POLL_MS = "10000";
    expect(getRuntimeConfig().healthPollIntervalMs).toBe(10_000);
  });

  it("treats 0 / false / off as health poll disabled", () => {
    process.env.EDGEQUAKE_HEALTH_POLL_MS = "0";
    expect(getRuntimeConfig().healthPollIntervalMs).toBe(false);
    process.env.EDGEQUAKE_HEALTH_POLL_MS = "false";
    expect(getRuntimeConfig().healthPollIntervalMs).toBe(false);
    process.env.EDGEQUAKE_HEALTH_POLL_MS = "off";
    expect(getRuntimeConfig().healthPollIntervalMs).toBe(false);
  });

  it("defaults showDevLoginHint to false", () => {
    delete process.env.NEXT_PUBLIC_SHOW_DEV_LOGIN_HINT;
    expect(getRuntimeConfig().showDevLoginHint).toBe(false);
  });

  it("enables showDevLoginHint when NEXT_PUBLIC_SHOW_DEV_LOGIN_HINT is true", () => {
    process.env.NEXT_PUBLIC_SHOW_DEV_LOGIN_HINT = "true";
    expect(getRuntimeConfig().showDevLoginHint).toBe(true);
  });
});

describe("parseHealthPollIntervalMs", () => {
  it("returns false for unset, 0, false, and off", () => {
    expect(parseHealthPollIntervalMs(undefined)).toBe(false);
    expect(parseHealthPollIntervalMs(0)).toBe(false);
    expect(parseHealthPollIntervalMs(false)).toBe(false);
    expect(parseHealthPollIntervalMs("off")).toBe(false);
    expect(parseHealthPollIntervalMs("false")).toBe(false);
  });

  it("returns a floor integer for a positive interval", () => {
    expect(parseHealthPollIntervalMs(10000)).toBe(10_000);
    expect(parseHealthPollIntervalMs("10000")).toBe(10_000);
    expect(parseHealthPollIntervalMs("1500.9")).toBe(1500);
  });
});
