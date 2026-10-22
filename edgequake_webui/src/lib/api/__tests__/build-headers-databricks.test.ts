/**
 * PDF upload uses XMLHttpRequest. Databricks Apps replaces Authorization, so
 * the EdgeQuake bearer must also be on X-EdgeQuake-Authorization.
 */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { buildHeaders } from "@/lib/api/client";
import { clearTokens, setTokens } from "@/lib/api/client-context";

vi.mock("@/lib/runtime-config", () => ({
  getRuntimeApiBaseUrl: () => "/api/v1",
  getRuntimeServerBaseUrl: () => "",
}));

describe("buildHeaders Databricks upload auth", () => {
  beforeEach(() => {
    const map = new Map<string, string>();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => (map.has(key) ? map.get(key)! : null),
      setItem: (key: string, value: string) => {
        map.set(key, String(value));
      },
      removeItem: (key: string) => {
        map.delete(key);
      },
      clear: () => map.clear(),
      get length() {
        return map.size;
      },
      key: (index: number) => Array.from(map.keys())[index] ?? null,
    });
    vi.stubGlobal("document", { cookie: "" });
  });

  afterEach(() => {
    clearTokens();
    vi.unstubAllGlobals();
  });

  it("copies the access bearer onto X-EdgeQuake-Authorization", () => {
    setTokens("eq-session-jwt");
    const headers = buildHeaders(undefined, new FormData());
    expect(headers.get("Authorization")).toBe("Bearer eq-session-jwt");
    expect(headers.get("X-EdgeQuake-Authorization")).toBe("Bearer eq-session-jwt");
    expect(headers.has("Content-Type")).toBe(false);
  });

  it("does not invent a bearer when the session is signed out", () => {
    clearTokens();
    const headers = buildHeaders();
    expect(headers.get("Authorization")).toBeNull();
    expect(headers.get("X-EdgeQuake-Authorization")).toBeNull();
  });
});
