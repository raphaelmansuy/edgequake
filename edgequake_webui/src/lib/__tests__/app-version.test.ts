import { describe, expect, it } from "vitest";
import packageInfo from "../../../package.json";
import { getAppVersion, getAppVersionNumber } from "../app-version";

describe("getAppVersion", () => {
  it("uses the current package version and always prefixes it with v", () => {
    expect(getAppVersion()).toBe(`v${packageInfo.version}`);
  });

  it("normalizes a provided version string without duplicating the prefix", () => {
    expect(getAppVersion("1.2.3")).toBe("v1.2.3");
    expect(getAppVersion("v1.2.3")).toBe("v1.2.3");
  });
});

describe("getAppVersionNumber", () => {
  it("returns the raw version without the leading v (for i18n interpolation)", () => {
    expect(getAppVersionNumber()).toBe(packageInfo.version);
  });

  it("strips a leading v from an explicit version", () => {
    expect(getAppVersionNumber("v1.2.3")).toBe("1.2.3");
    expect(getAppVersionNumber("1.2.3")).toBe("1.2.3");
  });
});
