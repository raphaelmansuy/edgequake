/**
 * SPEC-180 — visual verification of the image and profile UI.
 *
 * These tests exist to produce screenshots for human inspection, but they also assert the
 * behaviour each screenshot proves, so a regression fails here instead of surviving as a
 * stale picture:
 *
 *   - an image document renders the picture, with zoom/download/open, not its filename
 *   - the same image renders inside the Query source pane
 *   - the lineage tree does not claim entity extraction a chunk-only workspace never ran
 *
 * WHY `prepareSpec168Page` first: it installs the hermetic shell (setup status, health, model
 * eligibility, settings access). Without it those calls reach the dev server, the tenant guard
 * paints "Loading workspace…" over an already-rendered page, and every screenshot shows a
 * spinner that has nothing to do with the code under test.
 */
import { expect, test, type Page, type Route } from "@playwright/test";
import { prepareSpec168Page } from "../spec168/helpers/rich-mock";
import { SPEC155_TENANT, SPEC155_WORKSPACE } from "../spec155/helpers/mock-api";
import { applyTheme, setViewport, shot } from "./helpers/shots";

const json = (data: unknown, status = 200) => ({
  status,
  contentType: "application/json",
  body: JSON.stringify(data),
});

const IMAGE_ID = "33333333-3333-4333-8333-333333333333";
const IMAGE_NAME = "Screenshot_2026-10-31_at_3.30.53_PM__2_.png";

/** A picture with an obvious subject, so a cropped or squashed thumbnail is visible. */
function sceneSvg(): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="900" height="640" viewBox="0 0 900 640">
    <rect width="900" height="640" fill="#f7f6f3"/>
    <rect x="40" y="40" width="820" height="560" fill="none" stroke="#465ab4" stroke-width="4"/>
    <text x="90" y="150" font-family="Helvetica,Arial" font-size="54" fill="#1d1c1a">Quarterly revenue</text>
    <text x="90" y="230" font-family="Helvetica,Arial" font-size="30" fill="#57544e">North 42% · South 31% · East 27%</text>
    <rect x="120" y="360" width="150" height="200" fill="#5a6ec8"/>
    <rect x="310" y="240" width="150" height="320" fill="#5a6ec8"/>
    <rect x="500" y="300" width="150" height="260" fill="#5a6ec8"/>
  </svg>`;
}

/** A chunk-only (classical RAG) workspace: no graph capability at all. */
async function installNormalWorkspace(page: Page) {
  const workspace = {
    id: SPEC155_WORKSPACE.id,
    tenant_id: SPEC155_TENANT.id,
    name: "Demo007",
    slug: "demo007",
    is_active: true,
    rag_profile: "normal",
    embedding_modalities: "text,image",
    fts_regconfig: "english",
    rag_profile_capabilities: {
      extracts_graph: false,
      embeds_images: true,
      queries_graph: false,
      allows_graph_modes: false,
      shows_graph_ui: false,
    },
  };
  await page.route(/\/api\/v1\/tenants\/[^/]+\/workspaces(\?.*)?$/, (route: Route) =>
    route.fulfill(json({ items: [workspace], total: 1, offset: 0, limit: 20 })),
  );
  await page.route(/\/api\/v1\/workspaces\/[0-9a-z-]+(\?.*)?$/, (route: Route) =>
    route.fulfill(json(workspace)),
  );
}

const imageDoc = {
  id: IMAGE_ID,
  title: "Quarterly revenue",
  file_name: IMAGE_NAME,
  source_type: "image",
  mime_type: "image/png",
  file_size: 17_655,
  status: "completed",
  created_at: "2026-10-31T08:00:00Z",
  updated_at: "2026-10-31T08:00:02Z",
  chunk_count: 1,
  entity_count: 0,
  relationship_count: 0,
  tenant_id: SPEC155_TENANT.id,
  workspace_id: SPEC155_WORKSPACE.id,
};

/**
 * Register the image document's routes.
 *
 * Order matters, and in a way that bit us twice:
 *  - Playwright matches the LAST registered route first, so the specific `download/original`
 *    handler must be registered after any broader `download/**` one.
 *  - `prepareSpec168Page` installs `**\/api/v1/documents**`, which matches ANY url containing
 *    `documents/<uuid>` — including `/download/original` — and answers it with the document
 *    JSON. So every handler here must be registered AFTER that helper, and the image byte
 *    route must be the very last one.
 */
async function installImageDocument(page: Page) {
  await page.route("**/api/v1/documents/*/download/markdown", (route: Route) =>
    route.fulfill({ status: 200, contentType: "text/markdown", body: "# Report\n\nText." }),
  );
  await page.route("**/api/v1/documents/*/download/original", (route: Route) =>
    route.fulfill({ status: 200, contentType: "image/svg+xml", body: sceneSvg() }),
  );
  await page.route("**/api/v1/documents/*/pages/health**", (route: Route) =>
    route.fulfill({ json: { document_id: IMAGE_ID, page_count: 0, pages: [] } }),
  );
  await page.route("**/api/v1/documents/*/lineage**", (route: Route) =>
    route.fulfill({
      json: {
        document_id: IMAGE_ID,
        chunking_strategy: "fixed",
        avg_chunk_size: null,
        embedding_model: "gemini-embedding-2",
        embedding_dimensions: 1536,
        entity_count: 0,
        relationship_count: 0,
      },
    }),
  );
  await page.route("**/api/v1/documents?**", (route: Route) =>
    route.fulfill({
      json: {
        documents: [imageDoc],
        items: [imageDoc],
        total: 1,
        page: 1,
        page_size: 100,
        total_pages: 1,
        has_more: false,
        status_counts: { pending: 0, processing: 0, completed: 1, partial_failure: 0, failed: 0, cancelled: 0, unknown: 0 },
      },
    }),
  );
  // The document itself. WHY the suffix guard: this pattern also matches
  // `/download/original`, and answering that with document JSON made the <img> a broken icon.
  await page.route("**/api/v1/documents/" + IMAGE_ID + "**", (route: Route) => {
    const path = new URL(route.request().url()).pathname;
    if (/\/(download|pages|lineage|metadata|chunks|assets)\b/.test(path)) {
      return route.fallback();
    }
    return route.fulfill({ json: imageDoc });
  });
}

test.describe("@spec180 image and profile UI", () => {
  test.beforeEach(({}, info) => info.setTimeout(120_000));

  test("T-180-ui-image: a document image renders as the picture, not its filename", async ({ page }) => {
    await prepareSpec168Page(page, { empty: true });
    await installImageDocument(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(`/documents/${IMAGE_ID}`, { waitUntil: "domcontentloaded" });

    await expect(page.getByTestId("document-image-viewer")).toBeVisible();
    // WHY poll naturalWidth: the <img> is visible as soon as it mounts, but the blob is
    // written by a fetch. Screenshotting at "visible" once captured a broken-image icon.
    await expect
      .poll(
        async () =>
          page.evaluate(() => {
            const img = document.querySelector<HTMLImageElement>("[data-testid='document-image']");
            return img ? img.naturalWidth : 0;
          }),
        { timeout: 15_000 },
      )
      .toBeGreaterThan(0);

    // The bug: the filename was painted as the document body.
    await expect(page.locator("[data-testid='document-image-stage']")).not.toContainText(".png");
    await expect(page.getByTestId("document-image-zoom-level")).toHaveText("100%");
    await expect(page.getByTestId("document-image-download")).toHaveAttribute("download", IMAGE_NAME);
    await expect(page.getByTestId("document-image-open")).toBeVisible();
    // No tenant-guard spinner over a page that has already rendered.
    await expect(page.getByTestId("tenant-guard-overlay")).toHaveCount(0);

    await shot(page, "40-document-image-detail");

    await page.getByTestId("document-image-zoom-in").click();
    await expect(page.getByTestId("document-image-zoom-level")).toHaveText("125%");
    await shot(page, "41-document-image-zoomed");
    await page.getByTestId("document-image-fit").click();
    await expect(page.getByTestId("document-image-zoom-level")).toHaveText("100%");

    await applyTheme(page, "dark");
    await shot(page, "42-document-image-detail", { theme: "dark" });
    await applyTheme(page, "light");
    await setViewport(page, 390);
    await expect(page.getByTestId("document-image-viewer")).toBeVisible();
    await shot(page, "43-document-image-phone", { width: 390 });
  });

  test("T-180-ui-image: the query source pane shows a cited image as a picture", async ({ page }) => {
    await prepareSpec168Page(page, { empty: true });
    await installImageDocument(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    // Dock the companion pane the way a citation click does: `?pane=pdf&doc=<id>`
    // (companion-pane.ts `decodeCompanionSearch`).
    await page.goto(`/query?pane=pdf&doc=${IMAGE_ID}`, { waitUntil: "domcontentloaded" });

    // The pane must show the picture, never a text pane holding the filename.
    await expect(page.getByTestId("companion-source-image")).toBeVisible({ timeout: 20_000 });
    await expect(page.getByTestId("companion-source-text")).toHaveCount(0);
    await expect
      .poll(
        async () =>
          page.evaluate(() => {
            const img = document.querySelector<HTMLImageElement>(
              "[data-testid='companion-source-image'] [data-testid='document-image']",
            );
            return img ? img.naturalWidth : 0;
          }),
        { timeout: 15_000 },
      )
      .toBeGreaterThan(0);
    await shot(page, "45-query-source-image");
  });

  test("T-180-ui-image: a chunk-only workspace shows no knowledge-graph section", async ({ page }) => {
    await prepareSpec168Page(page, { empty: true });
    // Registered after the shell so it wins: this workspace is chunk-only.
    await installNormalWorkspace(page);
    await installImageDocument(page);
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto(`/documents/${IMAGE_ID}`, { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("document-image")).toBeVisible();

    // Classical RAG has no graph: "Entities 0 / Relations 0" reads as a failed extraction and
    // is pure noise on a workspace that never ran one.
    //
    // WHY these locators: the left nav legitimately has a "Knowledge Graph" LINK, so a bare
    // text match is the wrong assertion. The graph *section* and the *stat cards* are what
    // must be absent; the stat cards are the four-cell metric grid at the top of the sidebar.
    await expect(page.getByTestId("entity-relation-stats")).toHaveCount(0);
    await expect(page.getByTestId("key-stats-entities")).toHaveCount(0);
    await expect(page.getByTestId("key-stats-relations")).toHaveCount(0);
    // Chunks and Processed still apply to a chunk-only workspace.
    await expect(page.getByTestId("key-stats-chunks")).toBeVisible();
    await shot(page, "46-normal-workspace-image-details");
  });
});
