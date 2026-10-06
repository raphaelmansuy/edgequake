import { describe, expect, it } from "vitest";
import { panelSizePercent } from "@/lib/documents/panel-size-percent";

describe("panelSizePercent", () => {
  it("reads a numeric v4 getSize()", () => {
    expect(panelSizePercent(48)).toBe(48);
  });

  it("reads { asPercentage }", () => {
    expect(panelSizePercent({ asPercentage: 52, asPixels: 400 })).toBe(52);
  });

  it("rejects garbage", () => {
    expect(panelSizePercent(undefined)).toBeNull();
    expect(panelSizePercent("50%")).toBeNull();
    expect(panelSizePercent({ asPercentage: "nope" })).toBeNull();
  });
});
