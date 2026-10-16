import { describe, expect, it } from "vitest";
import { sigmaContainerBox } from "./container-box";

describe("sigmaContainerBox", () => {
  it("skips a pane that has not been laid out", () => {
    expect(sigmaContainerBox({ offsetWidth: 0, offsetHeight: 480 })).toBeNull();
    expect(sigmaContainerBox({ offsetWidth: 520, offsetHeight: 0 })).toBeNull();
  });

  it("returns the box once the pane has width and height", () => {
    expect(sigmaContainerBox({ offsetWidth: 520, offsetHeight: 640 })).toEqual({
      width: 520,
      height: 640,
    });
  });
});
