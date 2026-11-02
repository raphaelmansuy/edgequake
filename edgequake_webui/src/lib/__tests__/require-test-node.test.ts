import { describe, expect, it } from "vitest";
import { isSupportedNode } from "../../../scripts/require-test-node.mjs";

/**
 * The Node floor is load-bearing: below it, jsdom cannot load and every DOM test file dies before
 * running, while the suite still reports the *other* files as passed. These cases pin the boundary
 * that `engines.node` and `.nvmrc` both claim, so the three cannot drift apart.
 */
describe("isSupportedNode", () => {
  it.each(["22.13.0", "22.13.9", "22.20.1", "24.0.0", "24.14.1", "26.1.0"])(
    "accepts %s",
    (version: string) => {
      expect(isSupportedNode(version)).toBe(true);
    },
  );

  it.each(["20.14.0", "20.19.33", "22.0.0", "22.12.9", "23.11.0", "18.20.4"])(
    "refuses %s",
    (version: string) => {
      expect(isSupportedNode(version)).toBe(false);
    },
  );

  it("treats a leading v the way node reports it", () => {
    expect(isSupportedNode("v24.14.1")).toBe(true);
    expect(isSupportedNode("v20.14.0")).toBe(false);
  });
});
