/**
 * SPEC-180 screenshot helper. Writes under specs/180-normal-rag/e2e/screenshots/{SPEC180_ROUND}/.
 * Same naming as the SPEC-179 helper: `<name>-<theme>-<width>.png`.
 */
import fs from "node:fs";
import path from "node:path";
import type { Locator, Page } from "@playwright/test";
import { applyTheme, setViewport, type Theme } from "../../spec168/helpers/shots";

export { applyTheme, setViewport };
export type { Theme };

const REPO_ROOT = path.resolve(__dirname, "../../../../");
export const SPEC180_SHOTS_ROOT = path.join(REPO_ROOT, "specs/180-normal-rag/e2e/screenshots");
export const ROUND = process.env.SPEC180_ROUND ?? "round-01";

function target(name: string, theme: Theme, width: number): string {
  const dir = path.join(SPEC180_SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  return path.join(dir, `${name}-${theme}-${width}.png`);
}

/** Viewport screenshot (the app scrolls inside containers, so full-page would clip). */
export async function shot(
  page: Page,
  name: string,
  opts: { theme?: Theme; width?: number } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const file = target(name, theme, width);
  await page.screenshot({ path: file, animations: "disabled", caret: "hide", timeout: 15_000 });
  return file;
}

/** One element (a dialog, a card). */
export async function shotLocator(
  page: Page,
  element: Locator,
  name: string,
  opts: { theme?: Theme; width?: number } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const file = target(name, theme, width);
  await element.screenshot({ path: file, animations: "disabled", caret: "hide", timeout: 15_000 });
  return file;
}
