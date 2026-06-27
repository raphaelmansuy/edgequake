/**
 * SPEC-020 / FIX-DEV-PROXY — runtime config dev vs prod API URL resolution.
 */
import { afterEach, describe, expect, it } from "bun:test";
import { getRuntimeApiBaseUrl, getRuntimeConfig } from "../runtime-config";

const ORIGINAL_ENV = { ...process.env };

afterEach(() => {
  process.env = { ...ORIGINAL_ENV };
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
});
