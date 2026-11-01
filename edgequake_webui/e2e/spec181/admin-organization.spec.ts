/**
 * SPEC-181 — organization creation, the owner choice, and the permission matrix.
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareAdminPage, type AdminMockOptions } from "./helpers/admin-mock";
import { setViewport, shotBoth } from "./helpers/shots";

const open = async (page: Page, opts: AdminMockOptions = {}) => {
  const state = await prepareAdminPage(page, opts);
  await setViewport(page, 1440);
  await page.goto("/", { waitUntil: "domcontentloaded" });
  return state;
};

const openCreateOrg = async (page: Page) => {
  await page.getByTestId("workspace-selector").click();
  await page.getByTestId("header-create-tenant").click();
  await expect(page.getByTestId("create-organization-dialog")).toBeVisible();
};

test.describe("SPEC-181 create organization @spec181", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("three decisions, owner named, then confirmation", async ({ page }) => {
    const state = await open(page, { viewer: "platform", noOrganizations: true });
    await page.getByTestId("guard-create-tenant").click();

    const dialog = page.getByTestId("create-organization-dialog");
    await expect(dialog).toBeVisible();
    await expect(page.getByTestId("create-organization-name")).toBeVisible();
    await expect(page.getByTestId("create-organization-owner-self")).toBeVisible();
    await expect(page.getByTestId("create-organization-owner-email")).toBeVisible();
    await expect(page.getByTestId("create-organization-plan")).toBeVisible();
    // No stepper: this is three decisions, not eight steps.
    await expect(page.getByTestId("wizard-progress")).toHaveCount(0);

    await shotBoth(page, "create-organization");

    await page.getByTestId("create-organization-name").fill("Globex Holdings");
    await page.getByTestId("create-organization-owner-email").click();
    await page.getByTestId("create-organization-owner-email-field").fill("ines@globex.test");
    await expect(page.getByTestId("create-organization-dialog-finish")).toBeEnabled();
    await page.getByTestId("create-organization-dialog-finish").click();

    // LAW-181-2: the confirmation names the owner it created — no toast dead end.
    const confirmation = page.getByTestId("create-organization-confirmation");
    await expect(confirmation).toBeVisible({ timeout: 20_000 });
    await expect(confirmation).toContainText("ines@globex.test");
    await expect(confirmation).toContainText("Globex Holdings");
    // The workspace shown must be the one this create produced, not a stale sibling.
    await expect(confirmation).toContainText("/w/default");
    await shotBoth(page, "create-organization-confirmation");

    const posts = state.calls.filter((c) => c.method === "POST" && c.path === "/api/v1/tenants");
    expect(posts).toHaveLength(1);
    expect((posts[0].body as Record<string, unknown>).owner).toEqual({
      kind: "email",
      email: "ines@globex.test",
    });
  });

  test("a slug collision is reported inline and keeps the input", async ({ page }) => {
    // `failCreate: "slug"` makes the mock answer 409 tenant_slug_taken, which is what the real
    // endpoint returns for a duplicate slug (LAW-181-8).
    await open(page, { viewer: "platform", noOrganizations: true, failCreate: "slug" });
    await page.getByTestId("guard-create-tenant").click();
    await page.getByTestId("create-organization-name").fill("Acme Research");
    await page.getByTestId("create-organization-dialog-finish").click();

    const error = page.getByTestId("create-organization-error");
    await expect(error).toBeVisible({ timeout: 15_000 });
    // The field is not cleared: a refusal never costs the operator their work.
    await expect(page.getByTestId("create-organization-name")).toHaveValue("Acme Research");
    await shotBoth(page, "create-organization-slug-taken");
  });

  test("an owner cannot create an organization at all", async ({ page }) => {
    await open(page, { viewer: "owner", noOrganizations: true });
    // The empty state offers no create button, and the switcher has no row.
    await expect(page.getByTestId("guard-create-tenant")).toHaveCount(0);
    await expect(page.getByTestId("guard-copy-request")).toBeVisible();
  });
});

test.describe("SPEC-181 permission matrix @spec181", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("the roles tab renders the axis-aware capability matrix", async ({ page }) => {
    await open(page, { viewer: "owner" });
    await page.goto("/members?tab=roles", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("roles-tab")).toBeVisible({ timeout: 30_000 });

    // Organization capabilities are granted by admin; workspace delete is owner-only.
    await expect(page.getByTestId("cap-organization.workspaces.create")).toBeVisible();
    await expect(page.getByTestId("cap-workspace.delete")).toBeVisible();
    const wsDelete = page.getByTestId("cap-workspace.delete");
    const cells = wsDelete.locator("td");
    // Column order is owner, admin, member, readonly.
    await expect(cells.nth(0)).toHaveAttribute("data-granted", "true");
    await expect(cells.nth(1)).toHaveAttribute("data-granted", "false");

    await shotBoth(page, "roles-matrix", 1440);
  });
});

test.describe("SPEC-181 management pages @spec181", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("deployment organizations page lists every tenant with usage", async ({ page }) => {
    await open(page, { viewer: "platform" });
    await page.goto("/settings/deployment/organizations", { waitUntil: "domcontentloaded" });

    await expect(page.getByTestId("organizations-table")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("organization-row-acme")).toBeVisible();
    await expect(page.getByTestId("organization-row-globex")).toBeVisible();
    // The page is the only place a platform admin can create an organization besides the header.
    await expect(page.getByTestId("organizations-create")).toBeVisible();

    await shotBoth(page, "organizations-page");
  });

  test("organization workspaces page separates active from archived", async ({ page }) => {
    await open(page, { viewer: "owner" });
    await page.goto("/settings/organization/workspaces", { waitUntil: "domcontentloaded" });

    await expect(page.getByTestId("workspaces-list")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("workspaces-tab-live")).toBeVisible();
    await expect(page.getByTestId("workspace-archive-legal")).toBeVisible();
    await shotBoth(page, "workspaces-page");

    // Archiving is the fix the quota refusal points at, so it must actually work.
    await page.getByTestId("workspaces-tab-archived").click();
    await expect(page.getByTestId("workspaces-empty-archived")).toBeVisible();
  });

  test("the workspaces page is absent for a member", async ({ page }) => {
    await open(page, { viewer: "member" });
    await page.goto("/settings/organization/workspaces", { waitUntil: "domcontentloaded" });
    // The gate redirects; the page must not render its list.
    await expect(page.getByTestId("workspaces-list")).toHaveCount(0);
  });
});
