import { describe, expect, it } from "vitest";
import { isQueryMode, QUERY_MODES } from "@/types";

describe("QueryMode backend parity (UI-DRY-005)", () => {
  it("includes mix and bypass modes from backend", () => {
    expect(QUERY_MODES).toContain("mix");
    expect(QUERY_MODES).toContain("bypass");
  });

  it("validates known backend mode strings", () => {
    for (const mode of QUERY_MODES) {
      expect(isQueryMode(mode)).toBe(true);
    }
    expect(isQueryMode("invalid")).toBe(false);
  });
});
