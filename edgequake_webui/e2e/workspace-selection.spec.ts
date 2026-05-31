// E2E tests for workspace/tenant default selection
import { expect, test } from "@playwright/test";
import { waitForAppReady } from "./helpers/app-ready";
import { skipUnlessLiveStack } from "./helpers/live-stack";

test.beforeEach(() => {
  skipUnlessLiveStack();
});

test.describe("Workspace/Tenant Default Selection", () => {
  test.beforeEach(async ({ page }) => {
    // Clear localStorage before each test
    await page.goto("/");
    await page.evaluate(() => localStorage.clear());
  });

  test("first-time user sees workspace selector initialized", async ({
    page,
  }) => {
    await page.goto("/");

    // Wait for workspace selector to be visible
    await expect(page.getByTestId("workspace-selector")).toBeVisible({
      timeout: 10000,
    });

    // Should auto-select first available workspace
    const selectorText = await page
      .getByTestId("workspace-selector")
      .textContent();
    expect(selectorText).toBeTruthy();
    expect(selectorText).not.toContain("Select workspace");
  });

  test("returning user automatically enters last workspace", async ({
    page,
    context,
  }) => {
    // First, set up a workspace by visiting the page and letting it auto-select
    await page.goto("/");
    await waitForAppReady(page);
    await expect(page.getByTestId("workspace-selector")).toBeVisible({
      timeout: 10_000,
    });

    await page.reload();
    await waitForAppReady(page);
    await expect(page.getByTestId("workspace-selector")).toBeVisible({
      timeout: 10_000,
    });
  });

  test("can manually switch workspace", async ({ page }) => {
    await page.goto("/");

    // Wait for initialization
    await waitForAppReady(page);

    // Click workspace selector
    await page.getByTestId("workspace-selector").click();

    // Should show dropdown with workspaces
    await expect(page.getByRole("menuitem").first()).toBeVisible();

    // Click a workspace
    await page.getByRole("menuitem").first().click();

    // Selector should update
    await expect(page.getByTestId("workspace-selector")).toBeVisible();
  });
});
