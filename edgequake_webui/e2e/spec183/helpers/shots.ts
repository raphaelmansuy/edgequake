/**
 * SPEC-183 screenshot helper.
 *
 * Writes `<repo>/specs/183-workspace-image-fix/screenshots/<round>/`, using the same
 * `<name>-<theme>-<width>.png` convention as the SPEC-168/180 helpers so the artefacts sort and
 * read the same way. `<round>` comes from `SPEC183_ROUND` (default `round-01`).
 */
import fs from "node:fs";
import path from "node:path";
import type { Locator, Page } from "@playwright/test";
import { applyTheme, setViewport, type Theme } from "../../spec168/helpers/shots";

export { applyTheme, setViewport };
export type { Theme };

const REPO_ROOT = path.resolve(__dirname, "../../../..");
export const SPEC183_SHOTS_ROOT = path.join(
  REPO_ROOT,
  "specs/183-workspace-image-fix/screenshots",
);
export const ROUND = process.env.SPEC183_ROUND ?? "round-01";

function target(name: string, theme: Theme, width: number): string {
  const dir = path.join(SPEC183_SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  return path.join(dir, `${name}-${theme}-${width}.png`);
}

/** Viewport screenshot. The app scrolls inside containers, so full-page would clip. */
export async function spec183Shot(
  page: Page,
  name: string,
  opts: { theme?: Theme; width?: number } = {},
): Promise<string> {
  const theme = opts.theme ?? (await currentTheme(page));
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const file = target(name, theme, width);
  await page.screenshot({ path: file, animations: "disabled", caret: "hide", timeout: 15_000 });
  return file;
}

/** One element (a dialog, a notice). */
export async function spec183ShotLocator(
  page: Page,
  element: Locator,
  name: string,
  opts: { theme?: Theme; width?: number } = {},
): Promise<string> {
  const theme = opts.theme ?? (await currentTheme(page));
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const file = target(name, theme, width);
  await element.screenshot({ path: file, animations: "disabled", caret: "hide", timeout: 15_000 });
  return file;
}

/** The theme actually on screen, so a filename never claims "light" for a dark capture. */
async function currentTheme(page: Page): Promise<Theme> {
  const dark = await page.evaluate(() =>
    document.documentElement.classList.contains("dark"),
  );
  return dark ? "dark" : "light";
}
