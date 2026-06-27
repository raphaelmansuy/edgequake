/**
 * SPEC-021 P-G13 — runtime API URL injection for browser vs server.
 */
import { afterEach, describe, expect, it } from "bun:test";
import { resolveRuntimeApiUrlForInjection } from "../resolve-runtime-api-url";

const ORIGINAL_NODE_ENV = process.env.NODE_ENV;
const ORIGINAL_EDGEQUAKE_API_URL = process.env.EDGEQUAKE_API_URL;

afterEach(() => {
  process.env.NODE_ENV = ORIGINAL_NODE_ENV;
  if (ORIGINAL_EDGEQUAKE_API_URL === undefined) {
    delete process.env.EDGEQUAKE_API_URL;
  } else {
    process.env.EDGEQUAKE_API_URL = ORIGINAL_EDGEQUAKE_API_URL;
  }
});

describe("resolveRuntimeApiUrlForInjection", () => {
  it("returns empty string in development (same-origin dev proxy)", () => {
    process.env.NODE_ENV = "development";
    process.env.EDGEQUAKE_API_URL = "http://127.0.0.1:8081";
    expect(resolveRuntimeApiUrlForInjection()).toBe("");
  });

  it("returns EDGEQUAKE_API_URL in production", () => {
    process.env.NODE_ENV = "production";
    process.env.EDGEQUAKE_API_URL = "http://api.example.com";
    expect(resolveRuntimeApiUrlForInjection()).toBe("http://api.example.com");
  });
});
