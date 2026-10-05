/**
 * E2E — Document scope picker (SPEC-031) scroll + a11y.
 *
 * Regression coverage for the reported bug: with many documents the picker's
 * list was clipped and NOT scrollable (inner column used `h-full` against an
 * auto-height parent, so `overflow-y-auto` never engaged). Also covers the
 * 20-item cap (`has_more` "Load more") and keyboard navigation.
 *
 * Uses the UI-only mock stack (no live backend): `mockBackendForUiOnly` plus a
 * deterministic `/documents/search` stub returning 45 completed documents so the
 * list overflows and paging is exercised.
 */
import { expect, test, type Page } from "@playwright/test";
import { GOTO_OPTS } from "./helpers/app-ready";
import { mockBackendForUiOnly } from "./helpers/mock-backend";
import { expectScrollable, scrollToBottom } from "./helpers/scroll";

/** 45 completed documents — enough to overflow the bounded list and page. */
const ALL_DOCS = Array.from({ length: 45 }, (_, i) => ({
  id: `doc-${String(i + 1).padStart(3, "0")}`,
  title: `Document ${String(i + 1).padStart(3, "0")}`,
  status: "completed",
}));

/** Stub the type-ahead search endpoint with paging + has_more semantics. */
async function mockDocumentSearch(page: Page): Promise<void> {
  await page.route("**/api/v1/documents/search**", async (route) => {
    const url = new URL(route.request().url());
    const q = (url.searchParams.get("q") ?? "").toLowerCase();
    // Mirror the backend: page_size hard-capped at 50, no offset.
    const pageSize = Math.min(
      Number(url.searchParams.get("page_size") ?? "20") || 20,
      50,
    );
    const filtered = q
      ? ALL_DOCS.filter((d) => d.title.toLowerCase().includes(q))
      : ALL_DOCS;
    const items = filtered.slice(0, pageSize);
    await route.fulfill({
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        items,
        total: filtered.length,
        has_more: filtered.length > items.length,
      }),
    });
  });
}

async function openPicker(page: Page) {
  // SPEC-155/composer: empty-state trigger is `query-scope-chip` ("All docs").
  const trigger = page.getByTestId("query-scope-chip");
  await trigger.click();
  const list = page.getByTestId("scope-picker-list");
  await list.waitFor({ state: "visible" });
  // Wait for the first window of results to render.
  await page.getByTestId("scope-picker-option").first().waitFor();
  return { trigger, list };
}

test.describe("Document scope picker — scroll & a11y", () => {
  test.beforeEach(async ({ page }) => {
    // Constrain the viewport so the popover's max-height forces overflow.
    await page.setViewportSize({ width: 1000, height: 640 });
    await mockBackendForUiOnly(page);
    await mockDocumentSearch(page);
    await page.goto("/query", GOTO_OPTS);
    await page.waitForLoadState("domcontentloaded");
    await page.locator("main").first().waitFor({ state: "visible", timeout: 20_000 });
  });

  test("list is scrollable when results overflow (regression)", async ({
    page,
  }) => {
    const { list } = await openPicker(page);

    // First window is 20 items.
    await expect(page.getByTestId("scope-picker-option")).toHaveCount(20);

    // The core regression: the list must actually scroll, not clip.
    await expectScrollable(list);

    // Scrolling to the bottom reveals trailing content (the Load more row).
    await scrollToBottom(list);
    await expect(page.getByTestId("scope-picker-load-more")).toBeVisible();
  });

  test("load more pages through remaining documents", async ({ page }) => {
    await openPicker(page);
    await expect(page.getByTestId("scope-picker-option")).toHaveCount(20);

    await page.getByTestId("scope-picker-load-more").click();

    // 45 total — the larger window returns all of them, so Load more disappears.
    await expect(page.getByTestId("scope-picker-option")).toHaveCount(45);
    await expect(page.getByTestId("scope-picker-load-more")).toHaveCount(0);
    // Last document is now present in the DOM.
    await expect(
      page.getByTestId("scope-picker-option").nth(44),
    ).toContainText("Document 045");
  });

  test("keyboard navigation: focus, arrows, enter toggle, escape + focus return", async ({
    page,
  }) => {
    const { trigger } = await openPicker(page);

    // Radix onOpenAutoFocus moves focus into the search input (no setTimeout race).
    const search = page.getByTestId("scope-picker-search");
    await expect(search).toBeFocused();

    const options = page.getByTestId("scope-picker-option");
    // ArrowDown from search jumps to the first option.
    await page.keyboard.press("ArrowDown");
    await expect(options.first()).toBeFocused();
    // ArrowDown again moves to the second option.
    await page.keyboard.press("ArrowDown");
    await expect(options.nth(1)).toBeFocused();

    // Escape closes WITHOUT selecting — empty-state trigger is still mounted,
    // so focus returns to it.
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("scope-picker-list")).toHaveCount(0);
    await expect(trigger).toBeFocused();

    // Reopen and toggle a selection with Enter. Selecting the first document
    // remounts QueryScopeBar from empty → active (the empty-state trigger is
    // replaced by pills + Add). Assert selection via the scope pill.
    await openPicker(page);
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect(
      page.getByRole("button", { name: /remove document 001 from scope/i }),
    ).toBeVisible();
  });

  test("footer (clear all) stays reachable when the list is full and an item is selected", async ({
    page,
  }) => {
    // First selection remounts empty → active; reopen via the scope chip so
    // the popover stays open with an existing selection (footer renders).
    await openPicker(page);
    await page.getByTestId("scope-picker-option").first().click();
    await expect(
      page.getByRole("button", { name: /remove document 001 from scope/i }),
    ).toBeVisible({ timeout: 15_000 });

    // Selection can retain the open popover. Close it explicitly before
    // reopening so the trigger cannot toggle it shut during measurement.
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("scope-picker-list")).toHaveCount(0);
    // Composer always uses `query-scope-chip` as the popover trigger (no separate Add).
    await page.getByTestId("query-scope-chip").click();
    const list = page.getByTestId("scope-picker-list");
    await list.waitFor({ state: "visible", timeout: 15_000 });
    // Wait for the first page of results to paint before measuring scroll metrics
    // (CI can open the popover shell before options layout → scrollHeight 0).
    await expect(page.getByTestId("scope-picker-option")).toHaveCount(20, {
      timeout: 15_000,
    });
    await expectScrollable(list);

    // shrink-0 footer must remain visible (not clipped by overflow-hidden).
    await expect(page.getByTestId("scope-picker-clear-all")).toBeVisible({
      timeout: 10_000,
    });
    await expect(page.getByText(/1 selected/i)).toBeVisible();
  });
});
