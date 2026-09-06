/**
 * SPEC-146 — assert a control is not merely "visible" to Playwright but
 * has a usable box that intersects a scroll ancestor (or the viewport).
 * Fixes false greens when SecurityFields sit below documents-chrome max-h clip.
 *
 * Also provides containment helpers: child fully inside parent, no horizontal
 * overflow of a shell (wrap/shrink polish — not clip).
 */
import { expect, type Locator, type Page } from "@playwright/test";

const TOLERANCE_PX = 1;

export async function assertControlInView(
  locator: Locator,
  opts?: {
    /** Scroll ancestor test id (default: documents-chrome when present). */
    ancestorTestId?: string;
    minHeight?: number;
    page?: Page;
  },
): Promise<void> {
  const minHeight = opts?.minHeight ?? 24;
  await expect(locator).toBeVisible({ timeout: 20_000 });
  const box = await locator.boundingBox();
  expect(box, "control must have a bounding box").toBeTruthy();
  expect(box!.height).toBeGreaterThanOrEqual(minHeight);
  expect(box!.width).toBeGreaterThanOrEqual(minHeight);

  const ancestorTestId = opts?.ancestorTestId;
  if (ancestorTestId && opts?.page) {
    const ancestor = opts.page.getByTestId(ancestorTestId);
    if ((await ancestor.count()) > 0) {
      const aBox = await ancestor.boundingBox();
      expect(aBox, "ancestor must have a bounding box").toBeTruthy();
      const overlaps =
        box!.x < aBox!.x + aBox!.width &&
        box!.x + box!.width > aBox!.x &&
        box!.y < aBox!.y + aBox!.height &&
        box!.y + box!.height > aBox!.y;
      expect(
        overlaps,
        `control must intersect #${ancestorTestId} client rect (not clipped)`,
      ).toBe(true);
    }
  }
}

/**
 * Child bounding box must sit fully inside parent (1px tolerance).
 * Intersection-only is not enough — catches Vision past the dashed dropzone.
 */
export async function assertBoxInside(
  child: Locator,
  parent: Locator,
  label = "control",
): Promise<void> {
  await expect(child).toBeVisible({ timeout: 20_000 });
  await expect(parent).toBeVisible({ timeout: 20_000 });
  const c = await child.boundingBox();
  const p = await parent.boundingBox();
  expect(c, `${label} must have a bounding box`).toBeTruthy();
  expect(p, "parent must have a bounding box").toBeTruthy();
  expect(
    c!.x,
    `${label} left (${c!.x}) must be >= parent left (${p!.x})`,
  ).toBeGreaterThanOrEqual(p!.x - TOLERANCE_PX);
  expect(
    c!.y,
    `${label} top (${c!.y}) must be >= parent top (${p!.y})`,
  ).toBeGreaterThanOrEqual(p!.y - TOLERANCE_PX);
  expect(
    c!.x + c!.width,
    `${label} right (${c!.x + c!.width}) must be <= parent right (${p!.x + p!.width})`,
  ).toBeLessThanOrEqual(p!.x + p!.width + TOLERANCE_PX);
  expect(
    c!.y + c!.height,
    `${label} bottom (${c!.y + c!.height}) must be <= parent bottom (${p!.y + p!.height})`,
  ).toBeLessThanOrEqual(p!.y + p!.height + TOLERANCE_PX);
}

/** Shell must not scroll horizontally (controls wrap/shrink inside). */
export async function assertNoHorizontalOverflow(
  locator: Locator,
  label = "shell",
): Promise<void> {
  await expect(locator).toBeVisible({ timeout: 20_000 });
  const metrics = await locator.evaluate((el) => ({
    scrollWidth: el.scrollWidth,
    clientWidth: el.clientWidth,
  }));
  expect(
    metrics.scrollWidth,
    `${label} scrollWidth (${metrics.scrollWidth}) must be <= clientWidth (${metrics.clientWidth}) + ${TOLERANCE_PX}`,
  ).toBeLessThanOrEqual(metrics.clientWidth + TOLERANCE_PX);
}

/**
 * Parser + Vision triggers fully inside the dashed dropzone; dropzone itself
 * has no horizontal overflow. Call after DOC_ABAC chrome is ready.
 */
export async function assertDropzoneContainment(page: Page): Promise<void> {
  const dropzone = page.getByTestId("document-dropzone");
  await expect(dropzone).toBeVisible({ timeout: 20_000 });
  await assertNoHorizontalOverflow(dropzone, "document-dropzone");

  const parser = page.getByTestId("spec038-upload-parser-select");
  await assertBoxInside(parser, dropzone, "parser select");

  const vision = page.getByTestId("vision-settings-panel-trigger");
  if ((await vision.count()) > 0 && (await vision.isVisible())) {
    await assertBoxInside(vision, dropzone, "vision settings trigger");
  }
}

/** Expand SecurityFields and assert Classification + Share are usable in chrome. */
export async function assertSecurityFieldsUsable(page: Page): Promise<void> {
  await expect(page.getByTestId("spec146-security-fields")).toBeVisible({
    timeout: 20_000,
  });
  const toggle = page.getByTestId("spec146-security-toggle");
  if (await toggle.isVisible()) {
    const expanded = await toggle.getAttribute("aria-expanded");
    if (expanded !== "true") await toggle.click();
  }
  const classification = page.getByTestId("spec146-classification");
  const share = page.getByTestId("spec146-share-mode");
  await assertControlInView(classification, {
    page,
    ancestorTestId: "documents-chrome",
  });
  await assertControlInView(share, {
    page,
    ancestorTestId: "documents-chrome",
  });
  await expect(classification).toContainText(/Internal|Public|Confidential|Secret|Restricted/i);
  await expect(share).toContainText(/Workspace|ACL|Classified|Owner/i);
  await expect(page.getByTestId("spec146-export-control")).toBeVisible();
  await expect(page.getByTestId("spec146-pii")).toBeVisible();
}
