/**
 * SPEC-185 — probe the Documents page upload zone at the widths people actually use.
 *
 * Writes screenshots AND the measured geometry to JSON, so a layout claim in the fix is backed
 * by numbers (column widths, truncation, overflow) rather than by an impression of a picture.
 */
import { expect, test } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { openNormalWorkspace } from "./spec183/helpers/normal-workspace";
import { SPEC183_SHOTS_ROOT, ROUND } from "./spec183/helpers/shots";

const OUT_DIR = path.join(SPEC183_SHOTS_ROOT, ROUND);

async function measure(page: import("@playwright/test").Page, label: string) {
  // Wait for the real layout, not a frame with no workspace: measuring before the fetch lands
  // reports "no zone" or a 0px column as if it were a defect.
  const zone = page.getByTestId("document-dropzone");
  await zone.waitFor({ state: "visible", timeout: 20_000 });
  await page.waitForTimeout(500);
  const data = await page.evaluate(() => {
    const root = document.querySelector('[data-testid="document-dropzone"]');
    const notice = document.querySelector('[data-testid="dropzone-text-only-notice"]');
    const title = document.querySelector('[data-testid="document-dropzone-title"]');
    const box = (el: Element | null) => {
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return {
        w: Math.round(r.width),
        h: Math.round(r.height),
        x: Math.round(r.x),
        y: Math.round(r.y),
      };
    };
    const overflow = (el: Element | null) => {
      if (!el) return null;
      return {
        scrollW: el.scrollWidth,
        clientW: el.clientWidth,
        clippedX: el.scrollWidth > el.clientWidth + 1,
        scrollH: el.scrollHeight,
        clientH: el.clientHeight,
        clippedY: el.scrollHeight > el.clientHeight + 1,
      };
    };
    const clippedText: string[] = [];
    document
      .querySelectorAll('[data-testid="document-dropzone"] *')
      .forEach((el) => {
        if (el.children.length === 0 && el.textContent?.trim()) {
          if (el.scrollWidth > el.clientWidth + 1) {
            clippedText.push(el.textContent.trim().slice(0, 40));
          }
        }
      });
    const combo = document.querySelector('[data-testid="upload-parser-vision-combo"]');
    return {
      viewport: { w: window.innerWidth, h: window.innerHeight },
      zone: box(root),
      zoneOverflow: overflow(root),
      zoneLayout: root?.getAttribute("data-fill-layout") ?? null,
      zoneDensity: root?.getAttribute("data-density") ?? null,
      notice: box(notice),
      title: box(title),
      combo: box(combo),
      comboOverflow: overflow(combo),
      hasPopoverBar: Boolean(
        document.querySelector('[data-testid="upload-settings-popover-bar"]'),
      ),
      hasCompactTrigger: Boolean(
        document.querySelector('[data-testid="upload-settings-compact-trigger"]'),
      ),
      clippedText,
    };
  });
  return { label, ...data };
}

test.describe("@spec185 documents upload layout", () => {
  test.beforeEach(({}, info) => info.setTimeout(180_000));

  test("no upload-zone layout clips its own content at any width", async ({ page }) => {
    // The measured regression: a docked band held 113px of controls in a 76px box, so the
    // parser and vision selects were simply not on screen. Width alone never caught it, and a
    // width-only media query never could, because the layout is height-driven.
    for (const width of [1440, 1280, 1024, 820]) {
      await openNormalWorkspace(page, { path: "/documents", images: false });
      await page.setViewportSize({ width, height: 900 });
      await page.getByTestId("document-dropzone").waitFor({ state: "visible" });
      await page.waitForTimeout(700);

      for (const stage of ["closed", "preview"] as const) {
        if (stage === "preview") {
          const preview = page.getByTestId("right-panel-collapsed-bar");
          if ((await preview.count()) === 0) continue;
          await preview.click();
          await page.waitForTimeout(900);
        }
        const row = await measure(page, `assert-${stage}-${width}`);
        expect(row.zone, `no dropzone at ${width}/${stage}`).not.toBeNull();
        expect(
          row.zoneOverflow!.clippedY,
          `dropzone clips vertically at ${width}/${stage}: ` +
            `${row.zoneOverflow!.scrollH}px of content in a ${row.zoneOverflow!.clientH}px box`,
        ).toBe(false);
        // Nothing may be reachable only by scrolling a zone that is a drop target: a scroll
        // inside it competes with the click that opens the file picker.
        expect(
          row.zoneOverflow!.clippedX,
          `dropzone clips horizontally at ${width}/${stage}`,
        ).toBe(false);
      }
    }
  });

  test("the text-only note never takes the drop target's line", async ({ page }) => {
    await openNormalWorkspace(page, { path: "/documents", images: false });
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.getByTestId("document-dropzone").waitFor({ state: "visible" });
    await page.waitForTimeout(700);

    const row = await measure(page, "notice-outside");
    // 897px of a 963px line was the original defect: the selects were squeezed to 0px wide.
    expect(row.notice!.w).toBeLessThanOrEqual(row.zone!.w);
    // The settings must remain usable: either inline with real width, or behind one button.
    const settingsReachable =
      (row.combo !== null && row.combo.w > 200) || row.hasCompactTrigger || row.hasPopoverBar;
    expect(settingsReachable, "upload settings are not reachable").toBe(true);
  });

  test("capture the upload zone across widths with the preview panel open", async ({ page }) => {
    fs.mkdirSync(OUT_DIR, { recursive: true });
    const rows: unknown[] = [];

    for (const width of [1440, 1280, 1024, 820]) {
      await openNormalWorkspace(page, { path: "/documents", images: false });
      await page.setViewportSize({ width, height: 900 });
      await page.getByTestId("document-dropzone").waitFor({ state: "visible" });
      await page.waitForTimeout(800);

      rows.push(await measure(page, `closed-${width}`));
      await page.screenshot({
        path: path.join(OUT_DIR, `spec185-documents-${width}-closed.png`),
        animations: "disabled",
      });

      // Open the Preview panel: this is the narrow-column case in the report.
      const preview = page.getByTestId("right-panel-collapsed-bar");
      if ((await preview.count()) > 0) {
        await preview.click();
        await page.waitForTimeout(900);
        rows.push(await measure(page, `preview-${width}`));
        await page.screenshot({
          path: path.join(OUT_DIR, `spec185-documents-${width}-preview.png`),
          animations: "disabled",
        });
      }
    }

    fs.writeFileSync(
      path.join(OUT_DIR, "spec185-measurements.json"),
      JSON.stringify(rows, null, 2),
    );
  });
});
