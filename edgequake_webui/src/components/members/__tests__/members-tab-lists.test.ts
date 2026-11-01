import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const SOURCE = readFileSync(
  resolve(__dirname, "../members-tab.tsx"),
  "utf8",
);

/**
 * The members page renders two list queries: `grants` (SPEC-181) and `workspaces`.
 *
 * Both come from endpoints that answer with a JSON array, and both were mapped with `?.map`
 * or `?? []` — neither of which protects against a 200 whose body is an *object*. That is not
 * hypothetical: the e2e hermetic guard answers any unmocked `/api/v1/**` request with
 * `{ error: { … } }`, so a spec that merely navigated to `/members` crashed the whole page
 * with `grantRows.data?.map is not a function` and took 24 unrelated specs down with it.
 *
 * A missing list is not a reason to lose the page. These assertions keep the guard in place.
 */
describe("members tab list rendering", () => {
  it("does not call .map on the grants payload directly", () => {
    expect(SOURCE).not.toMatch(/grantRows\.data\?\.map/);
  });

  it("does not assume the workspace payload is an array", () => {
    expect(SOURCE).not.toMatch(/workspaceOptions\.data \?\? \[\]\)\.map/);
  });

  it("guards a non-array grants payload before mapping it", () => {
    expect(SOURCE).toMatch(/Array\.isArray\(grantRows\.data\) \? grantRows\.data : \[\]/);
  });

  it("guards a non-array workspace payload before mapping it", () => {
    expect(SOURCE).toMatch(
      /Array\.isArray\(workspaceOptions\.data\) \? workspaceOptions\.data : \[\]/,
    );
  });
});
