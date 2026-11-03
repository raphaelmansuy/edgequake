/**
 * SPEC-180 screen contract — every UI promise the pack makes, asserted in one place.
 *
 * WHY this file exists: the pack's promises are spread across
 * [11-ux-ui-spec](../../../specs/180-normal-rag/11-ux-ui-spec.md) and
 * [12-screens](../../../specs/180-normal-rag/12-screens.md), and the existing specs covered the
 * query composer and the graph empty state but not the documents table, the upload chrome, the
 * read-only retrieval card, or the nav. A contract that is not asserted is a paragraph.
 *
 * Hermetic: the SPEC-168 shell plus the SPEC-183 normal-workspace fixture, so it runs in the UI
 * gate and needs no backend.
 */
import { expect, test } from "@playwright/test";
import { openNormalWorkspace } from "./spec183/helpers/normal-workspace";
import { spec183Shot, spec183ShotLocator } from "./spec183/helpers/shots";

test.describe("@spec180 screen contract", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("LAW-180-11: the nav drops the graph links instead of disabling them", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/documents", images: false });
    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 20_000 });

    // Absent, not disabled: a greyed row is still an invitation to a broken screen. The
    // locator is scoped to the sidebar's own link list, because Next.js also emits
    // `<link rel="prefetch" href="/graph">` in `<head>` and a bare `a[href]` query matches the
    // document's links, not the navigation's.
    // The sidebar's own `<nav aria-label="Navigation">`. Scoping matters twice here: Next.js
    // emits `<link rel="prefetch" href="/graph">` into `<head>`, so a bare `a[href]` query would
    // match the document's links rather than the navigation's.
    const nav = page.getByRole("navigation", { name: /^Navigation$/ });
    // `toHaveCount` retries: a bare `.count()` reads the DOM once and can race the first paint,
    // which is exactly how a passing screen looked like a failing one during this audit.
    await expect(nav.locator('a[href="/graph"]')).toHaveCount(0);
    await expect(nav.locator('a[href="/knowledge"]')).toHaveCount(0);
    // The workspace itself is still reachable, so the section was not emptied by accident.
    await expect(nav.locator('a[href="/workspace"]')).toHaveCount(1);
  });

  test("documents: Chunks replaces the graph counts, and zero entities is not an error", async ({
    page,
  }) => {
    await openNormalWorkspace(page, { path: "/documents", images: false });
    await expect(page.getByText("MANIFEST.md").first()).toBeVisible({ timeout: 20_000 });

    const headers = await page.locator("thead").first().innerText();
    expect(headers).toMatch(/Chunks/);
    // A normal workspace has no entity counts: showing "0 entities" reads as a failure state.
    expect(headers).not.toMatch(/Entit/i);
    expect(headers).not.toMatch(/Relation/i);
    await spec183Shot(page, "20-spec180-documents-chunks-column");
  });

  test("upload: no extraction-mode select, and the text-only limit is stated", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/documents", images: false });
    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 20_000 });

    // Extraction is not a per-document choice here (LAW-180-13), so no extraction-mode control
    // is mounted at all. Asserted by test-id prefix so a rename cannot quietly pass this.
    await expect(page.locator('[data-testid*="extraction-mode"]')).toHaveCount(0);
    // EC-180-18 fails before the request, with the fix named.
    await expect(page.getByTestId("dropzone-text-only-notice")).toBeVisible();
  });

  test("query: Chunks and Chat only, kind chips, and no image search on text evidence", async ({
    page,
  }) => {
    await openNormalWorkspace(page, { path: "/query", images: false });
    await expect(page.getByTestId("query-composer")).toBeVisible({ timeout: 20_000 });

    await page.getByTestId("query-mode-selector").click();
    const items = page.locator("[data-testid^=query-mode-][data-mode]");
    await expect(items).toHaveCount(2);
    expect(await items.allInnerTexts()).toEqual([
      expect.stringMatching(/Chunks/),
      expect.stringMatching(/Chat/),
    ]);
    // The graph modes are absent, not merely disabled (LAW-180-8).
    for (const hidden of ["mix", "local", "global", "hybrid"]) {
      await expect(page.getByTestId(`query-mode-${hidden}`)).toHaveCount(0);
    }
    await page.keyboard.press("Escape");

    await expect(page.getByTestId("query-kind-chips")).toBeVisible();
    // `embeds_images` is false, so the picture affordance is not offered (EC-180-18).
    await expect(page.getByTestId("query-search-image-button")).toHaveCount(0);
    await spec183Shot(page, "21-spec180-query-composer-text-only");
  });

  test("query: an image workspace offers Search by image", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/query", images: true });
    await expect(page.getByTestId("query-composer")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("query-search-image-button")).toBeVisible();
    await expect(page.getByTestId("query-kind-chips")).toBeVisible();
  });

  test("/knowledge explains the absence instead of drawing an empty canvas", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/knowledge", images: false });

    await expect(page.getByTestId("no-graph-state")).toBeVisible({ timeout: 20_000 });
    const copy = await page.getByTestId("no-graph-state").innerText();
    expect(copy).toMatch(/no knowledge graph/i);
    // A way out, so the dead end is not dead.
    await expect(page.getByTestId("no-graph-documents")).toBeVisible();
    await spec183Shot(page, "22-spec180-knowledge-empty-state");
  });

  test("the retrieval card is text, not a disabled form", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/workspace", images: false });
    const card = page.getByTestId("workspace-retrieval-card");
    await expect(card).toBeVisible({ timeout: 20_000 });

    // The three facts the card exists to state.
    await expect(page.getByTestId("workspace-retrieval-type")).toContainText(/Normal/i);
    await expect(page.getByTestId("workspace-retrieval-evidence")).toContainText(/Text/i);
    await expect(page.getByTestId("workspace-retrieval-language")).toBeVisible();
    // Immutability is stated, not implied.
    await expect(page.getByTestId("workspace-retrieval-permanent")).toContainText(/Permanent/i);
    // A disabled input still looks editable; this card has no control at all.
    expect(await card.locator("button, input, select, textarea, [role=radio]").count()).toBe(0);

    // Graph-only surfaces are absent (LAW-180-2).
    await expect(page.getByTestId("workspace-extraction-mode-card")).toHaveCount(0);
    await expect(page.getByTestId("rebuild-knowledge-graph-button")).toHaveCount(0);

    await spec183ShotLocator(page, card, "23-spec180-retrieval-card-readonly");
  });

  test("an image workspace states its evidence on the card", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/workspace", images: true });
    await expect(page.getByTestId("workspace-retrieval-evidence")).toContainText(
      /Text and images/i,
      { timeout: 20_000 },
    );
  });
});
