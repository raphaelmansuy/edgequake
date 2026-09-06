/**
 * SPEC-146 containment scan — dashboard shells must not overflow horizontally
 * at tablet width. Portaled menus may leave the shell; in-flow chrome must not.
 */
import { expect, test, type Page } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import {
  assertProductChrome,
  setupSpec146Ui,
} from "./helpers/spec146-mocks";
import {
  assertDropzoneContainment,
  assertNoHorizontalOverflow,
  assertSecurityFieldsUsable,
} from "./helpers/spec146-visibility";

async function assertShellIfPresent(
  page: Page,
  testId: string,
): Promise<void> {
  const shell = page.getByTestId(testId);
  if ((await shell.count()) === 0) return;
  if (!(await shell.first().isVisible())) return;
  await assertNoHorizontalOverflow(shell.first(), testId);
}

test.describe("SPEC-146 containment scan", () => {
  test("dashboard shells @ tablet-768", async ({ page }) => {
    test.setTimeout(120_000);
    await page.setViewportSize({ width: 768, height: 1024 });
    await setupSpec146Ui(page);

    // Documents — dropzone is the proven overflow surface
    await page.goto("/documents", GOTO_OPTS);
    await assertProductChrome(page);
    await assertSecurityFieldsUsable(page);
    await assertDropzoneContainment(page);
    await assertShellIfPresent(page, "documents-chrome");
    await assertShellIfPresent(page, "documents-page-shell");
    await assertShellIfPresent(page, "spec099-primary-toolbar");
    await assertShellIfPresent(page, "document-filters");

    // Query
    await page.goto("/query", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.locator("main")).toBeVisible({ timeout: 20_000 });
    await assertShellIfPresent(page, "query-page-header");
    await assertShellIfPresent(page, "query-mode-selector");

    // Graph
    await page.goto("/graph", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.locator("main")).toBeVisible({ timeout: 20_000 });
    await assertShellIfPresent(page, "graph-header");

    // Settings (authz cluster)
    await page.goto("/settings", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.locator("main")).toBeVisible({ timeout: 20_000 });
    await assertShellIfPresent(page, "spec146-authz-settings");
    await assertShellIfPresent(page, "spec146-attrs-card");
    await assertShellIfPresent(page, "spec146-break-glass-card");

    // Workspace
    await page.goto("/workspace", GOTO_OPTS);
    await assertProductChrome(page);
    await expect(page.locator("main")).toBeVisible({ timeout: 20_000 });
    await assertNoHorizontalOverflow(page.locator("main"), "workspace-main");
  });
});
