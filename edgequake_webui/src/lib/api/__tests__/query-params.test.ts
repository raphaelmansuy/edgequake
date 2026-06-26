import { describe, expect, it } from "vitest";
import { buildQueryString, withQuery } from "../query-params";

describe("buildQueryString", () => {
  it("returns empty string for undefined / null params", () => {
    expect(buildQueryString(undefined)).toBe("");
    expect(buildQueryString({})).toBe("");
  });

  it("skips undefined and null values", () => {
    expect(buildQueryString({ a: 1, b: undefined, c: null, d: "x" })).toBe(
      "?a=1&d=x",
    );
  });

  it("keeps false and 0 (valid query values)", () => {
    expect(buildQueryString({ active: false, page: 0 })).toBe(
      "?active=false&page=0",
    );
  });

  it("serializes arrays as repeated keys", () => {
    expect(buildQueryString({ tags: ["a", "b", "c"] })).toBe(
      "?tags=a&tags=b&tags=c",
    );
  });

  it("skips undefined/null items inside arrays", () => {
    expect(
      buildQueryString({ tags: ["a", undefined, null, "b"] as unknown[] }),
    ).toBe("?tags=a&tags=b");
  });

  it("stringifies values with String()", () => {
    expect(buildQueryString({ n: 42, b: true })).toBe("?n=42&b=true");
  });
});

describe("withQuery", () => {
  it("returns the path when query is empty", () => {
    expect(withQuery("/documents", "")).toBe("/documents");
  });

  it("appends query with no extra separator", () => {
    expect(withQuery("/documents", "?page=1")).toBe("/documents?page=1");
  });
});
