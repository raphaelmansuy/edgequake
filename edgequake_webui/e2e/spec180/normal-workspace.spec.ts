/**
 * SPEC-180 — what a Normal workspace looks like across the app (16-e2e-test-matrix T-180-ui-*).
 * Hermetic: SPEC-168 rich mock with the workspace served as profile `normal`.
 */
import { expect, test, type Page } from "@playwright/test";
import { openWorkspacePage, switchWorkspace, type ChatCapture } from "./helpers/normal-fixture";
import { applyTheme, setViewport, shot, shotLocator } from "./helpers/shots";

const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
  "base64",
);

const composerReady = (page: Page) => expect(page.getByTestId("query-composer")).toBeVisible({ timeout: 20_000 });

/** The hermetic offer lists one chat model; the workspace default is not in it, so pick it once. */
async function ensureModel(page: Page) {
  const chip = page.getByTestId("query-model-chip");
  await expect(chip).toBeVisible();
  if (!/Pick a model/i.test((await chip.textContent()) ?? "")) return;
  await chip.click();
  await page.getByTestId("query-model-option").first().click();
  await expect(page.getByTestId("query-model-menu")).toBeHidden();
}

async function sendQuestion(page: Page, text: string) {
  await ensureModel(page);
  await page.getByTestId("query-composer").locator("textarea").fill(text);
  await page.getByTestId("query-send").click();
}

test.describe("@spec180 normal workspace", () => {
  test("T-180-ui-knowledge: nav hides the graph; /graph and /knowledge explain why", async ({ page }) => {
    await openWorkspacePage(page, { profile: "normal", images: true, path: "/graph" });
    await expect(page.getByTestId("no-graph-state")).toBeVisible();
    await expect(page.getByTestId("no-graph-documents")).toBeVisible();
    await expect(page.getByRole("link", { name: /Knowledge Graph/ })).toHaveCount(0);
    await expect(page.getByRole("link", { name: /^Knowledge$/ })).toHaveCount(0);
    await shot(page, "01-graph-no-graph-state");

    await page.goto("/knowledge");
    await expect(page.getByTestId("no-graph-state")).toBeVisible();
    await shot(page, "02-knowledge-no-graph-state");
  });

  test("T-180-ui-query: Chunks and Chat only, kind chips, search by image", async ({ page }) => {
    const chats: ChatCapture[] = [];
    await openWorkspacePage(page, { profile: "normal", images: true, path: "/query", chats });
    await composerReady(page);

    await page.getByTestId("query-mode-selector").click();
    await expect(page.getByTestId("query-mode-naive")).toBeVisible();
    await expect(page.getByTestId("query-mode-mix")).toHaveCount(0);
    await expect(page.getByTestId("query-mode-local")).toHaveCount(0);
    await expect(page.getByTestId("query-mode-global")).toHaveCount(0);
    await shot(page, "03-query-mode-menu");
    await page.keyboard.press("Escape");

    const chips = page.getByTestId("query-kind-chips");
    await expect(chips).toBeVisible();
    await expect(page.getByTestId("query-search-image-button")).toBeVisible();
    await shot(page, "04-query-composer-normal");

    // A kind filter rides in document_filter.doc_kinds.
    await page.getByTestId("query-kind-pdf").click();
    await sendQuestion(page, "What is the refund policy?");
    await expect.poll(() => chats.length).toBe(1);
    const first = JSON.parse(chats[0]!.body);
    expect(first.mode).toBe("naive");
    expect(first.document_filter?.doc_kinds).toEqual(["pdf"]);
    await expect(page.getByText("The refund clause is on page 4")).toBeVisible();
    await shot(page, "05-query-answer-with-chunks");

    // Image sources come back as a grid of thumbnails, strongest first.
    const grid = page.getByTestId("query-image-matches");
    await expect(grid).toBeVisible();
    await expect(page.getByTestId("query-image-match")).toHaveCount(5);
    await expect(grid.locator("img")).toHaveCount(5);
    await expect
      .poll(() => grid.locator("img").first().evaluate((img: HTMLImageElement) => img.naturalWidth))
      .toBeGreaterThan(0);

    // Search by image goes out as multipart with the picture in `query_image`.
    await page.getByTestId("query-search-image-input").setInputFiles({
      name: "counter.png",
      mimeType: "image/png",
      buffer: PNG,
    });
    await expect(page.getByTestId("query-search-image-chip")).toBeVisible();
    await shot(page, "06-query-search-image-attached");
    await sendQuestion(page, "Find pages that look like this");
    await expect.poll(() => chats.length).toBe(2);
    expect(chats[1]!.contentType).toMatch(/multipart\/form-data/);
    expect(chats[1]!.body).toContain('name="query_image"');
    expect(chats[1]!.body).toContain('name="request"');
    expect(chats[1]!.body).toContain("doc_kinds");
    await expect(page.getByTestId("query-image-matches")).toHaveCount(1);
    await shot(page, "06b-query-search-image-results-grid");
    await shotLocator(page, page.getByTestId("query-image-matches"), "06c-image-matches-grid-crop");
  });

  test("T-180-ui-query: a text-only normal workspace offers no image search", async ({ page }) => {
    await openWorkspacePage(page, { profile: "normal", images: false, path: "/query" });
    await composerReady(page);
    await expect(page.getByTestId("query-kind-chips")).toBeVisible();
    await expect(page.getByTestId("query-search-image-button")).toHaveCount(0);
    await shot(page, "07-query-composer-text-only");
  });

  test("T-180-ui-documents: Chunks column replaces graph counts", async ({ page }) => {
    await openWorkspacePage(page, { profile: "normal", images: true, path: "/documents" });
    await expect(page.getByText("handbook.pdf").first()).toBeVisible({ timeout: 20_000 });
    await expect(page.getByRole("columnheader", { name: /Chunks/ })).toBeVisible();
    await expect(page.getByRole("columnheader", { name: /Entities/ })).toHaveCount(0);
    await shot(page, "08-documents-chunks-column");
  });

  test("T-180-ui-settings: the Retrieval card states the permanent choice", async ({ page }) => {
    await openWorkspacePage(page, { profile: "normal", images: true, path: "/workspace" });
    const card = page.getByTestId("workspace-retrieval-card");
    await expect(card).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("workspace-retrieval-type")).toContainText(/Normal/i);
    await expect(page.getByTestId("workspace-retrieval-evidence")).toContainText(/image/i);
    await expect(page.getByTestId("workspace-retrieval-permanent")).toBeVisible();
    await shot(page, "09-workspace-retrieval-card");
    await shotLocator(page, card, "10-workspace-retrieval-card-crop");
  });

  test("T-180-ui-reconfigure: a Normal workspace edits models and parsing, never extraction", async ({ page }) => {
    await openWorkspacePage(page, { profile: "normal", images: true, path: "/workspace" });
    await page.getByTestId("workspace-edit-config").click();
    const wizard = page.getByTestId("reconfigure-workspace-wizard");
    await expect(wizard).toBeVisible({ timeout: 15_000 });
    await expect(wizard).not.toContainText(/Extraction|Budget/);
    await expect(wizard).toContainText("Models");
    await shotLocator(page, wizard, "15-reconfigure-normal-models");
  });

  test("T-180-ui-switch: switching to a graph workspace restores graph nav and modes", async ({ page }) => {
    await openWorkspacePage(page, { profile: "normal", images: true, path: "/query" });
    await composerReady(page);
    await expect(page.getByTestId("query-kind-chips")).toBeVisible();

    await switchWorkspace(page, { profile: "graph" });
    await composerReady(page);
    await expect(page.getByTestId("query-kind-chips")).toHaveCount(0);
    await page.getByTestId("query-mode-selector").click();
    await expect(page.getByTestId("query-mode-mix")).toBeVisible();
    await expect(page.getByTestId("query-mode-local")).toBeVisible();
    await shot(page, "11-query-graph-workspace-modes");
    await page.keyboard.press("Escape");
    await expect(page.getByRole("link", { name: /Knowledge Graph/ })).toBeVisible();
  });

  test("T-180-ui-image-grid: tiles stay balanced on phone and desktop, in light and dark", async ({ page }) => {
    await openWorkspacePage(page, { profile: "normal", images: true, path: "/query", width: 390 });
    await composerReady(page);
    await sendQuestion(page, "Find pages that look like this");
    const tiles = page.getByTestId("query-image-match");
    await expect(tiles).toHaveCount(5);
    await expect(tiles.last().locator("img")).toBeVisible();
    // Phone: 3 + 2, never 3 + 1 + 1. Rows are the distinct tile tops.
    const tops = await tiles.evaluateAll((els) => [...new Set(els.map((e) => Math.round(e.getBoundingClientRect().top)))]);
    expect(tops).toHaveLength(2);
    const firstRow = await tiles.evaluateAll((els) => els.filter((e) => Math.abs(e.getBoundingClientRect().top - els[0]!.getBoundingClientRect().top) < 2).length);
    expect(firstRow).toBe(3);
    await page.getByTestId("query-image-matches").evaluate((el) => el.scrollIntoView({ block: "center" }));
    await shot(page, "16-image-grid-phone", { width: 390 });

    await setViewport(page, 1440);
    await expect(tiles.first()).toBeVisible();
    const desktopTops = await tiles.evaluateAll((els) => [...new Set(els.map((e) => Math.round(e.getBoundingClientRect().top)))]);
    expect(desktopTops).toHaveLength(1);
    await applyTheme(page, "dark");
    await shot(page, "17-image-grid-desktop", { theme: "dark" });
  });

  test("T-180-ui-dark-phone: dark theme and a 390px phone frame", async ({ page }) => {
    await openWorkspacePage(page, { profile: "normal", images: true, path: "/query" });
    await composerReady(page);
    await applyTheme(page, "dark");
    await shot(page, "12-query-normal", { theme: "dark" });

    await setViewport(page, 390);
    await page.goto("/query");
    await composerReady(page);
    await expect(page.getByText("85 passages")).toBeVisible();
    await applyTheme(page, "light");
    await shot(page, "13-query-normal-phone", { width: 390 });
    await page.goto("/graph");
    await expect(page.getByTestId("no-graph-state")).toBeVisible();
    await shot(page, "14-graph-no-graph-phone", { width: 390 });
  });
});
