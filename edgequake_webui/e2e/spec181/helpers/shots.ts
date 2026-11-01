/**
 * SPEC-181 screenshot helper — the visual-inspection loop.
 *
 * Writes under `specs/181-admin/e2e/screenshots/<SPEC181_ROUND>/`, so every run leaves
 * comparable evidence across rounds, themes and widths. Theme and viewport helpers are shared
 * with SPEC-168/169 (DRY).
 */

import fs from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import { applyTheme, setViewport, type Theme } from "../../spec168/helpers/shots";

export { applyTheme, setViewport };
export type { Theme };

const REPO_ROOT = path.resolve(__dirname, "../../../../");
export const SPEC181_SHOTS_ROOT = path.join(REPO_ROOT, "specs/181-admin/e2e/screenshots");

export const ROUND = process.env.SPEC181_ROUND ?? "round-00-baseline";

export function shotPath(name: string, theme: Theme, width: number): string {
  const dir = path.join(SPEC181_SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  return path.join(dir, `${name}-${theme}-${width}.png`);
}

export async function shot(
  page: Page,
  name: string,
  opts: { theme?: Theme; width?: number; fullPage?: boolean } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? (page.viewportSize()?.width ?? 1440);
  const file = shotPath(name, theme, width);
  const take = () =>
    page.screenshot({
      path: file,
      fullPage: opts.fullPage ?? false,
      animations: "disabled",
      caret: "hide",
      timeout: 15_000,
    });
  try {
    await take();
  } catch {
    await page.waitForTimeout(500);
    await take();
  }
  return file;
}

/** Capture the same screen in light and dark at one width. */
export async function shotBoth(page: Page, name: string, width = 1440): Promise<void> {
  await applyTheme(page, "light");
  await page.waitForTimeout(200);
  await shot(page, name, { theme: "light", width });
  await applyTheme(page, "dark");
  await page.waitForTimeout(250);
  await shot(page, name, { theme: "dark", width });
  await applyTheme(page, "light");
}
