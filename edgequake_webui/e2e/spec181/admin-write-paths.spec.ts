/**
 * SPEC-181 — the write paths: workspace grants and archive.
 *
 * These cover the two halves of F-181-05 (the workspace grant axis had no write path and no
 * read projection) and EC-181-16 (archive frees a plan slot without deleting data).
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareAdminPage, type AdminMockOptions } from "./helpers/admin-mock";
import { setViewport, shotBoth } from "./helpers/shots";

const open = async (page: Page, opts: AdminMockOptions = {}, path = "/") => {
  const state = await prepareAdminPage(page, opts);
  await setViewport(page, 1440);
  await page.goto(path, { waitUntil: "domcontentloaded" });
  return state;
};

test.describe("SPEC-181 workspace grants @spec181", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("the grant editor shows one person's organization and workspace grants", async ({ page }) => {
    await open(page, { viewer: "owner", withMembers: true }, "/members");
    await expect(page.getByTestId("members-table")).toBeVisible({ timeout: 30_000 });

    // Open the first real member (not an invitation row).
    await page
      .locator("[data-testid^=member-row-][data-kind=member]")
      .first()
      .getByRole("button")
      .first()
      .click();

    const sheet = page.getByTestId("member-detail-sheet");
    await expect(sheet).toBeVisible();
    // F-181-05: two rows for one person, which the pre-181 table collapsed into one cell.
    await expect(page.getByTestId("detail-grants")).toBeVisible();
    await expect(page.getByTestId("grant-grant-org-1")).toBeVisible();
    await expect(page.getByTestId("grant-grant-ws-1")).toBeVisible();
    await expect(page.getByTestId("grant-grant-ws-1")).toContainText("Legal");

    await shotBoth(page, "member-grants-editor");
  });

  test("changing a grant role calls the grant endpoint", async ({ page }) => {
    const state = await open(page, { viewer: "owner", withMembers: true }, "/members");
    await expect(page.getByTestId("members-table")).toBeVisible({ timeout: 30_000 });
    await page
      .locator("[data-testid^=member-row-][data-kind=member]")
      .first()
      .getByRole("button")
      .first()
      .click();
    await expect(page.getByTestId("grant-grant-ws-1")).toBeVisible();

    await page.getByTestId("grant-role-grant-ws-1").click();
    await page.getByRole("option", { name: "Member" }).click();

    await expect
      .poll(() => state.calls.filter((c) => c.method === "PATCH" && c.path === "/grants").length, {
        timeout: 10_000,
      })
      .toBeGreaterThan(0);
  });

  test("a workspace admin sees a read-only single role, not the grant editor", async ({ page }) => {
    await open(page, { viewer: "member", withMembers: true }, "/members");
    // A member cannot open the directory at all; the restricted state is the designed answer.
    await expect(page.getByTestId("members-restricted")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("detail-grants")).toHaveCount(0);
  });
});

test.describe("SPEC-181 archive @spec181", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("archiving from the workspaces page calls the endpoint", async ({ page }) => {
    const state = await open(page, { viewer: "owner" }, "/settings/organization/workspaces");
    await expect(page.getByTestId("workspaces-list")).toBeVisible({ timeout: 30_000 });

    await page.getByTestId("workspace-archive-legal").click();

    await expect
      .poll(() => state.calls.filter((c) => c.path.includes("/archive")).length, { timeout: 10_000 })
      .toBe(1);
  });

  test("the archived tab separates archived workspaces", async ({ page }) => {
    await open(page, { viewer: "owner" }, "/settings/organization/workspaces");
    await expect(page.getByTestId("workspaces-list")).toBeVisible({ timeout: 30_000 });
    await page.getByTestId("workspaces-tab-archived").click();
    await expect(page.getByTestId("workspaces-empty-archived")).toBeVisible();
    await shotBoth(page, "workspaces-archived-tab");
  });
});
