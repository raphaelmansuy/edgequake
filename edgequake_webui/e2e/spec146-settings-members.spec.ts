/**
 * SPEC-146 — settings members / authz PAP cards when DOC_ABAC on.
 * Members use user Select (not UUID paste).
 */
import { expect, test } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";

const DEMO_MEMBERS = [
  {
    workspace_id: "ws",
    principal_kind: "user",
    principal_id: "alice-user-id",
    role_id: "role-viewer",
    role_name: "viewer",
  },
  {
    workspace_id: "ws",
    principal_kind: "user",
    principal_id: "bob-user-id",
    role_id: "role-editor",
    role_name: "editor",
  },
];

test.describe("SPEC-146 settings members", () => {
  test("authz settings section renders members Selects", async ({ page }) => {
    test.setTimeout(60_000);
    await setupSpec146Ui(page, { members: DEMO_MEMBERS });
    await page.goto("/settings", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.locator("main")).toBeVisible({ timeout: 20_000 });

    const authz = page.getByTestId("spec146-authz-settings");
    await expect(authz).toBeVisible({ timeout: 30_000 });
    const text = (await authz.innerText()).toLowerCase();
    expect(text).toMatch(/member|role|policy|security|attribute|break-glass/);
    expect(text).not.toContain("user uuid");
    expect(text).not.toContain("n hidden");
    expect(text).not.toContain("not_in_allow_set");
    expect(text).not.toContain("document:list_meta");

    await expect(page.getByTestId("spec146-member-user")).toBeVisible();
    await expect(page.getByTestId("spec146-member-role")).toBeVisible();
    await expect(page.getByTestId("spec146-members-skeleton")).toHaveCount(0);
    await expect(page.getByTestId("spec146-members-list")).toBeVisible();
    await expect(page.getByTestId("spec146-members-list")).toContainText(/alice/i);
    await expect(page.getByTestId("spec146-members-list")).toContainText(/bob/i);
    await expect(page.getByTestId("spec146-member-user")).toBeEnabled();
    // Attributes + Break-glass are first-class cards (not buried under Advanced).
    await expect(page.getByTestId("spec146-attrs-card")).toBeVisible();
    await expect(page.getByTestId("spec146-break-glass-card")).toBeVisible();
    await expect(page.getByTestId("spec146-authz-advanced")).toHaveCount(0);
    await expect(page.getByTestId("spec146-roles-create-form")).toHaveCount(0);
    await expect(page.getByTestId("spec146-members-list")).toContainText(/secret/i);
    await expect(page.getByTestId("spec146-members-list")).toContainText(/internal/i);
    await expect(page.getByTestId("spec146-members-list")).toContainText(/department:eng/i);
    await expect(page.getByTestId("spec146-member-invite-hint")).toBeVisible();
    await page.getByTestId("spec146-attrs-card").scrollIntoViewIfNeeded();
    await expect(page.getByTestId("spec146-attrs-card")).toBeInViewport();
    await page.getByTestId("spec146-break-glass-card").scrollIntoViewIfNeeded();
    await expect(page.getByTestId("spec146-break-glass-card")).toBeInViewport();
    const main = (await page.locator("main").innerText()).toLowerCase();
    expect(main).not.toContain("spec-146");
    expect(main).not.toContain("spec038");
  });
});
