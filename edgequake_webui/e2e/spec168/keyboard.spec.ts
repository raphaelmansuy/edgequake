/**
 * SPEC-168 keyboard contract — every destination, palette, help, focus ring.
 * Hermetic (mock-api project). Run: pnpm exec playwright test e2e/spec168/keyboard.spec.ts --project=mock-api
 */
import { expect, test, type Page } from "@playwright/test";
import { prepareSpec168Page } from "./helpers/rich-mock";

const CHORDS: Array<[string, string]> = [
  ["h", "/"],
  ["d", "/documents"],
  ["q", "/query"],
  ["g", "/graph"],
  ["k", "/knowledge"],
  ["w", "/workspace"],
  ["p", "/pipeline"],
  ["c", "/costs"],
  ["a", "/api-explorer"],
  ["s", "/settings"],
];

async function open(page: Page, route: string) {
  await prepareSpec168Page(page, {});
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(route, { waitUntil: "domcontentloaded" });
  await page.locator("#main-content").waitFor();
  // Hydration: the chord handler is mounted by a client effect.
  await page.waitForTimeout(800);
}

/** Park focus on <body> like a user who just clicked empty space. */
async function blurAll(page: Page) {
  await page.evaluate(() => (document.activeElement as HTMLElement | null)?.blur());
}

async function pathnameIs(page: Page, path: string) {
  await expect.poll(() => new URL(page.url()).pathname, { timeout: 10_000 }).toBe(path);
}

test.describe("SPEC-168 keyboard @spec168", () => {
  test.beforeEach(async ({ page }) => {
    test.setTimeout(90_000);
    await open(page, "/settings");
  });

  test("g-chords reach every destination", async ({ page }) => {
    for (const [key, path] of CHORDS) {
      await page.goto("/costs", { waitUntil: "domcontentloaded" });
      await page.locator("#main-content").waitFor();
      await page.waitForTimeout(500);
      await blurAll(page);
      await page.keyboard.press("g");
      await expect(page.getByTestId("chord-hint")).toBeVisible();
      await page.keyboard.press(key);
      await pathnameIs(page, path);
      await expect(page.getByTestId("chord-hint")).toHaveCount(0);
    }
  });

  test("chord popup lists all ten destinations and expires", async ({ page }) => {
    await blurAll(page);
    await page.keyboard.press("g");
    const hint = page.getByTestId("chord-hint");
    await expect(hint.locator("li")).toHaveCount(10);
    await expect(hint).toHaveCount(0, { timeout: 3_000 });
  });

  test("chords and single keys are ignored while typing", async ({ page }) => {
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    const search = page.getByPlaceholder(/search documents/i).first();
    await search.waitFor();
    await search.click();
    await page.keyboard.type("gq[?");
    await expect(search).toHaveValue("gq[?");
    expect(new URL(page.url()).pathname).toBe("/documents");
    await expect(page.getByTestId("keyboard-shortcuts-dialog")).toHaveCount(0);
  });

  test("Ctrl+K and Meta+K toggle the palette; Enter navigates", async ({ page }) => {
    for (const combo of ["Control+k", "Meta+k"]) {
      await page.keyboard.press(combo);
      const input = page.getByTestId("command-palette-input");
      await expect(input).toBeFocused();
      await page.keyboard.press(combo);
      await expect(input).toHaveCount(0);
    }
    await page.keyboard.press("Control+k");
    await page.getByTestId("command-palette-input").fill("pipeline");
    await page.keyboard.press("Enter");
    await pathnameIs(page, "/pipeline");
  });

  test("palette lists every destination with chord hints and Esc restores focus", async ({ page }) => {
    const trigger = page.getByTestId("header-search-trigger");
    await trigger.focus();
    await page.keyboard.press("Enter");
    const input = page.getByTestId("command-palette-input");
    await expect(input).toBeFocused();
    for (const name of ["Dashboard", "Documents", "Query", "Knowledge Graph", "Knowledge", "Workspace", "Pipeline", "Costs", "API Explorer", "Settings"]) {
      await expect(page.getByRole("option", { name: new RegExp(`^${name}`) }).first()).toBeVisible();
    }
    await page.keyboard.press("Escape");
    await expect(input).toHaveCount(0);
    await expect(trigger).toBeFocused();
  });

  test("? opens the truthful help dialog; Esc closes it", async ({ page }) => {
    await blurAll(page);
    await page.keyboard.press("?");
    const dialog = page.getByTestId("keyboard-shortcuts-dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.locator("kbd", { hasText: /^G$/ }).first()).toBeVisible();
    await expect(dialog).toContainText("Settings");
    await expect(dialog).not.toContainText("⌘G");
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
  });

  test("single keys do not fire under an open dialog", async ({ page }) => {
    await blurAll(page);
    await page.keyboard.press("?");
    await expect(page.getByTestId("keyboard-shortcuts-dialog")).toBeVisible();
    await page.keyboard.press("g");
    await page.keyboard.press("d");
    expect(new URL(page.url()).pathname).toBe("/settings");
  });

  test("[ toggles the sidebar between rail and expanded", async ({ page }) => {
    const sidebar = page.getByTestId("sidebar");
    await expect(sidebar).toHaveAttribute("data-collapsed", "false");
    const wide = (await sidebar.boundingBox())!.width;
    await blurAll(page);
    await page.keyboard.press("[");
    await expect(sidebar).toHaveAttribute("data-collapsed", "true");
    await expect.poll(async () => (await sidebar.boundingBox())!.width).toBeLessThan(wide);
    await page.keyboard.press("[");
    await expect(sidebar).toHaveAttribute("data-collapsed", "false");
  });

  test("collapsed rail can always be expanded again by button (wide)", async ({ page }) => {
    const sidebar = page.getByTestId("sidebar");
    const toggle = page.getByTestId("sidebar-toggle");
    await toggle.click();
    await expect(sidebar).toHaveAttribute("data-collapsed", "true");
    await expect(toggle).toBeVisible();
    await expect(toggle).toHaveAccessibleName(/expand sidebar/i);
    await toggle.click();
    await expect(sidebar).toHaveAttribute("data-collapsed", "false");
    await expect.poll(async () => (await sidebar.boundingBox())!.width).toBeGreaterThan(200);
  });

  test("below lg the rail expands into the navigation drawer (button and [)", async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 800 });
    const open = page.getByTestId("sidebar-open-drawer");
    await expect(open).toBeVisible();
    await open.click();
    const drawer = page.getByRole("dialog");
    await expect(drawer.getByRole("link", { name: "Documents" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(drawer).toHaveCount(0);
    await expect(open).toBeFocused();
    await blurAll(page);
    await page.keyboard.press("[");
    await expect(page.getByRole("dialog").getByRole("link", { name: "Documents" })).toBeVisible();
  });

  test("graph: ⌘K opens only the palette, / opens node search", async ({ page }) => {
    await page.goto("/graph?stream=0", { waitUntil: "domcontentloaded" });
    await page.locator("[data-graph-container], canvas").first().waitFor({ timeout: 20_000 });
    await page.waitForTimeout(800);
    await blurAll(page);
    await page.keyboard.press("Control+k");
    await expect(page.getByTestId("command-palette-input")).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(1);
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("command-palette-input")).toHaveCount(0);
    await blurAll(page);
    await page.keyboard.press("/");
    await expect(page.getByRole("dialog", { name: /search nodes/i })).toBeVisible();
  });

  test("skip link is the first tab stop and moves focus to main", async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.locator("#main-content").waitFor();
    await blurAll(page);
    await page.keyboard.press("Tab");
    const skip = page.getByRole("link", { name: /skip to/i }).first();
    await expect(skip).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator("#main-content")).toBeFocused();
  });

  test("every sidebar link is reachable by Tab and shows a visible focus ring", async ({ page }) => {
    await blurAll(page);
    const nav = page.getByRole("navigation", { name: /^navigation$/i });
    const links = nav.getByRole("link");
    expect(await links.count()).toBe(10);
    await links.first().focus();
    for (let i = 0; i < 10; i++) {
      const focused = page.locator(":focus");
      const ring = await focused.evaluate((el) => {
        const cs = getComputedStyle(el);
        return `${cs.boxShadow} ${cs.outlineStyle} ${cs.outlineWidth}`;
      });
      expect(ring, `link #${i} focus ring`).toMatch(/rgb|[1-9]px/);
      await page.keyboard.press("Tab");
    }
  });

  test("active destination is marked aria-current", async ({ page }) => {
    const nav = page.getByRole("navigation", { name: /^navigation$/i });
    await expect(nav.locator('a[aria-current="page"]')).toHaveAttribute("href", "/settings");
  });
});
