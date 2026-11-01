import { describe, expect, it } from "vitest";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

/**
 * SPEC-144 pinned Next.js to one exact version so the app could not drift onto an untested
 * release mid-cycle. That intent is still right, but a frozen literal turns every security
 * upgrade into a failing test that invites someone to "fix" it by reverting the patch.
 *
 * This guard states the durable rules instead:
 *
 * 1. `next` and `eslint-config-next` are exact-pinned and identical — the two must never skew.
 * 2. The installed version is at least the security floor: 16.4.1 is the first release without
 *    the unauthenticated RCE advisories (GHSA-673q-mjm6-r5xr, GHSA-2464-fhr4-w9xw,
 *    GHSA-vcvr-r3jv-pc5j). Going below it is a regression, not a pin.
 */
const SECURITY_FLOOR = [16, 4, 1] as const;

function isAtLeast(version: string, floor: readonly [number, number, number]): boolean {
  const parts = version.split(".").map((p) => Number.parseInt(p, 10));
  for (let i = 0; i < floor.length; i += 1) {
    const v = parts[i] ?? 0;
    if (v > floor[i]) return true;
    if (v < floor[i]) return false;
  }
  return true;
}

describe("SPEC-144 Next.js pin", () => {
  const pkg = require("../../../../package.json") as {
    dependencies: Record<string, string>;
    devDependencies: Record<string, string>;
  };

  it("exact-pins next and eslint-config-next to the same version", () => {
    const next = pkg.dependencies.next;
    const eslintNext = pkg.devDependencies["eslint-config-next"];
    // A caret/tilde range would let a lockfile refresh change the framework under us.
    expect(next, "next must be an exact pin").toMatch(/^\d+\.\d+\.\d+$/);
    expect(eslintNext, "eslint-config-next must be an exact pin").toMatch(/^\d+\.\d+\.\d+$/);
    expect(eslintNext).toBe(next);
  });

  it("never drops below the patched security floor", () => {
    expect(isAtLeast(pkg.dependencies.next, SECURITY_FLOOR)).toBe(true);
  });

  it("installs the version package.json pins", () => {
    const nextPkg = require("next/package.json") as { version: string };
    expect(nextPkg.version).toBe(pkg.dependencies.next);
    expect(isAtLeast(nextPkg.version, SECURITY_FLOOR)).toBe(true);
  });
});
