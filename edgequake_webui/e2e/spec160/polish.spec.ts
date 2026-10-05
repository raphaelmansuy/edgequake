/**
 * SPEC-160 polish captures — other languages, dark theme, narrow screens.
 * Each shot is opened and judged by eye; the asserts only guard against layout breaks.
 */
import { expect, test, type Page } from "@playwright/test";
import {
  createWorkspace,
  deleteWorkspace,
  openOnBackend,
  requireLiveBackend,
  settle,
  shot,
  type TestWorkspace,
} from "./support";

requireLiveBackend();

let workspace: TestWorkspace;
test.beforeAll(async () => {
  workspace = await createWorkspace("polish", { extraction_mode: "decision", decision_enabled: true });
});
test.afterAll(async () => {
  if (workspace) await deleteWorkspace(workspace.id);
});
test.afterEach(async ({ page }) => {
  await settle(page);
});

/** No control may spill out of its card: a horizontal scrollbar is a layout bug. */
async function expectNoHorizontalOverflow(page: Page, testId: string) {
  const overflow = await page.getByTestId(testId).evaluate((el) => el.scrollWidth - el.clientWidth);
  expect(overflow, `${testId} overflows by ${overflow}px`).toBeLessThanOrEqual(1);
}

const BAR_PARTS = [
  "document-dropzone-browse",
  "upload-extraction-mode-select",
  "spec038-upload-parser-select",
  "decision-status-indicator",
];

/** The bar's controls must sit inside the viewport and never overlap each other. */
async function expectBarWithoutOverlap(page: Page) {
  const viewport = page.viewportSize()!;
  const boxes: Array<[string, { x: number; y: number; width: number; height: number }]> = [];
  for (const id of BAR_PARTS) {
    const part = page.getByTestId(id).first();
    const box = (await part.count()) > 0 ? await part.boundingBox({ timeout: 2_000 }) : null;
    if (box) boxes.push([id, box]);
  }
  expect(boxes.map(([id]) => id), "bar parts rendered").toEqual(expect.arrayContaining(["upload-extraction-mode-select", "spec038-upload-parser-select"]));
  for (const [id, b] of boxes) {
    expect(b.x, `${id} left edge`).toBeGreaterThanOrEqual(0);
    expect(b.x + b.width, `${id} right edge`).toBeLessThanOrEqual(viewport.width + 1);
  }
  // Nothing may be clipped by the drop zone: parts sit inside it, or the zone scrolls to them.
  const dropzone = page.getByTestId("document-dropzone");
  const zone = (await dropzone.boundingBox())!;
  const scrolls = await dropzone.evaluate((el) => getComputedStyle(el).overflowY === "auto");
  for (const [id, b] of boxes) {
    if (!scrolls) {
      expect(b.y + b.height, `${id} bottom is inside the drop zone`).toBeLessThanOrEqual(zone.y + zone.height + 1);
    }
    expect(b.x + b.width, `${id} right is inside the drop zone`).toBeLessThanOrEqual(zone.x + zone.width + 1);
  }
  await expect(page.getByTestId("decision-status-indicator").first()).toHaveText(
    "Cannot reach the decision backend at localhost:11434.",
  );
  await expect(page.getByTestId("upload-extraction-mode-select")).not.toContainText("Extraction");
  await expect(page.getByTestId("upload-extraction-mode-select")).not.toContainText("closed questions");
  for (const id of ["upload-extraction-mode-select", "spec038-upload-parser-select"] as const) {
    const chevronFits = await page.getByTestId(id).evaluate((el) => {
      const svg = el.querySelector("svg");
      if (!svg) return false;
      const icon = svg.getBoundingClientRect();
      const box = el.getBoundingClientRect();
      return icon.width > 4 && icon.right <= box.right + 1 && icon.left >= box.left - 1;
    });
    expect(chevronFits, `${id} chevron is clipped`).toBe(true);
  }
  for (let i = 0; i < boxes.length; i++) {
    for (let j = i + 1; j < boxes.length; j++) {
      const [ida, a] = boxes[i];
      const [idb, b] = boxes[j];
      const apart = a.x + a.width <= b.x + 1 || b.x + b.width <= a.x + 1 || a.y + a.height <= b.y + 1 || b.y + b.height <= a.y + 1;
      expect(apart, `${ida} overlaps ${idb}: ${JSON.stringify([a, b])}`).toBe(true);
    }
  }
}

test.describe("SPEC-160 polish @spec160", () => {
  test("P01 card in French", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 1000 });
    await openOnBackend(page, workspace, "/workspace", { locale: "fr", decisionStatus: "model_missing" });
    const card = page.getByTestId("workspace-extraction-mode-card");
    await expect(card).toContainText("Mode d’extraction", { timeout: 60_000 });
    await expect(card).toContainText("Moteur Décision");
    await expect(page.getByTestId("decision-model")).toBeVisible();
    await expectNoHorizontalOverflow(page, "workspace-extraction-mode-card");
    await shot(page, "p01-card-fr", card);
  });

  test("P01b card in Chinese", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 1000 });
    await openOnBackend(page, workspace, "/workspace", { locale: "zh" });
    const card = page.getByTestId("workspace-extraction-mode-card");
    await expect(card).toContainText("提取模式", { timeout: 60_000 });
    await expect(card).toContainText("许可协议"); // the server's English notice is localized
    await expect(card).not.toContainText("Tev1 weights license");
    await shot(page, "p01-card-zh", card);
  });

  test("P03 card on a phone", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openOnBackend(page, workspace, "/workspace", { decisionStatus: "unreachable" });
    const card = page.getByTestId("workspace-extraction-mode-card");
    await expect(card).toBeVisible({ timeout: 60_000 });
    await expectNoHorizontalOverflow(page, "workspace-extraction-mode-card");
    await shot(page, "p03-card-390", card);
  });

  test("P04 upload bar on a phone and a tablet", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openOnBackend(page, workspace, "/documents", { decisionStatus: "unreachable" });
    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("upload-extraction-mode")).toBeVisible();
    await expect(page.getByTestId("decision-status-indicator")).toBeVisible();
    await expectBarWithoutOverlap(page);
    await shot(page, "p04-upload-390");
    await page.setViewportSize({ width: 820, height: 1000 });
    // The intake band is short here, so its controls stack and the band scrolls: reach them.
    await expect(async () =>
      page.getByTestId("upload-extraction-mode-select").scrollIntoViewIfNeeded({ timeout: 2_000 }),
    ).toPass({ timeout: 10_000 });
    await expect(page.getByTestId("upload-extraction-mode")).toBeVisible();
    await expect(page.getByTestId("decision-status-indicator")).toBeVisible();
    await expectBarWithoutOverlap(page);
    await shot(page, "p04-upload-820");
  });
});

test.describe("SPEC-160 dark theme @spec160", () => {
  test.use({ colorScheme: "dark" });

  test("P02 card and upload bar in dark", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 1000 });
    await openOnBackend(page, workspace, "/workspace");
    const card = page.getByTestId("workspace-extraction-mode-card");
    await expect(card).toBeVisible({ timeout: 60_000 });
    await expect(page.getByTestId("decision-status-indicator")).toHaveAttribute("data-state", "ready");
    await shot(page, "p02-card-dark", card);
    await page.goto("/documents", { waitUntil: "domcontentloaded" });
    await expect(page.getByTestId("document-dropzone")).toBeVisible({ timeout: 60_000 });
    await shot(page, "p02-upload-dark", page.getByTestId("document-dropzone"));
  });
});
