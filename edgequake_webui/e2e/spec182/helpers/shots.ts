/**
 * SPEC-182 screenshot helper.
 *
 * Writes to `specs/182-rags/screenshots/{SPEC182_ROUND}/`. Same naming convention as the
 * SPEC-180 helper (`<name>-<theme>-<width>.png`) so a reviewer can diff these against the
 * earlier packs without relearning the layout.
 */
import fs from "node:fs";
import path from "node:path";
import type { Locator, Page } from "@playwright/test";

const REPO_ROOT = path.resolve(__dirname, "../../../../");
export const SHOTS_ROOT = path.join(REPO_ROOT, "specs/182-rags/screenshots");
export const ROUND = process.env.SPEC182_ROUND ?? "round-01";

export type Theme = "light" | "dark";

function target(name: string, theme: Theme, width: number): string {
  const dir = path.join(SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  return path.join(dir, `${name}-${theme}-${width}.png`);
}

export async function applyTheme(page: Page, theme: Theme): Promise<void> {
  await page.evaluate((t) => {
    document.documentElement.classList.toggle("dark", t === "dark");
    localStorage.setItem("theme", t);
  }, theme);
}

export async function setViewport(page: Page, width: number): Promise<void> {
  const height = width <= 480 ? 812 : width <= 900 ? 1024 : 900;
  await page.setViewportSize({ width, height });
}

/**
 * Viewport screenshot.
 *
 * WHY not fullPage by default: the app scrolls inside containers. A full-page capture
 * stitches those containers at their scroll origin, which produces a tall image with the
 * interesting middle missing — the exact failure that made an earlier pack's screenshots
 * look broken.
 */
export async function shot(
  page: Page,
  name: string,
  opts: { theme?: Theme; width?: number } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const file = target(name, theme, width);
  const take = () =>
    page.screenshot({ path: file, animations: "disabled", caret: "hide", timeout: 20_000 });
  try {
    await take();
  } catch {
    // Headless Chrome GPU compositing can stall a frame; one settle + retry.
    await page.waitForTimeout(600);
    await take();
  }
  return file;
}

/** One element: a card, a source pane, a composer. */
export async function shotLocator(
  page: Page,
  element: Locator,
  name: string,
  opts: { theme?: Theme; width?: number } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const file = target(name, theme, width);
  await element.screenshot({ path: file, animations: "disabled", caret: "hide", timeout: 20_000 });
  return file;
}
