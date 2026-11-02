import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { MOCK_PRODUCT_VERSION } from "../../../e2e/spec155/helpers/mock-api";

const repoRoot = resolve(__dirname, "../../../..");

/**
 * The mocked `/health` feeds the version shown in every screenshot this fixture takes.
 *
 * It used to be the literal `0.28.5`, so a release review saw `API v0.28.5` next to
 * `UI v0.39.0` and had to guess which half was wrong. These assertions keep the mock
 * honest against the two files that actually define a release.
 */
describe("SPEC-155 mock product version", () => {
  it("matches the repo VERSION file", () => {
    const version = readFileSync(resolve(repoRoot, "VERSION"), "utf8").trim();
    expect(MOCK_PRODUCT_VERSION).toBe(version);
  });

  it("matches the shipped frontend package version", () => {
    const pkg = JSON.parse(
      readFileSync(resolve(repoRoot, "edgequake_webui/package.json"), "utf8"),
    ) as { version: string };
    expect(MOCK_PRODUCT_VERSION).toBe(pkg.version);
  });

  it("is a plain semver triple, so the header renders it verbatim", () => {
    expect(MOCK_PRODUCT_VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
