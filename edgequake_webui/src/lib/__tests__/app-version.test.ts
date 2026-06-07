import { describe, expect, it } from "vitest";
import packageInfo from "../../../package.json";
import { getAppVersion } from "../app-version";

describe("getAppVersion", () => {
  it("uses the current package version and always prefixes it with v", () => {
    expect(getAppVersion()).toBe(`v${packageInfo.version}`);
  });

  it("normalizes a provided version string without duplicating the prefix", () => {
    expect(getAppVersion("1.2.3")).toBe("v1.2.3");
    expect(getAppVersion("v1.2.3")).toBe("v1.2.3");
  });
});
