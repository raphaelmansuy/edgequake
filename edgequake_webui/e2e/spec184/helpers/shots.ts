/**
 * SPEC-184 screenshot helper — writes to `<repo>/specs/184-improve-provider-settings/e2e/`.
 *
 * Same `<name>-<theme>-<width>.png` convention as SPEC-168/177/183 so the rounds sort and read the
 * same way. `<round>` comes from `SPEC184_ROUND` (default `round-01`).
 */
import fs from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";
import { applyTheme, setViewport, type Theme } from "../../spec168/helpers/shots";

export { applyTheme, setViewport };
export type { Theme };

const REPO_ROOT = path.resolve(__dirname, "../../../..");
export const SPEC184_SHOTS_ROOT = path.join(
  REPO_ROOT,
  "specs/184-improve-provider-settings/e2e",
);
export const ROUND = process.env.SPEC184_ROUND ?? "round-01";

export async function shot(
  page: Page,
  name: string,
  opts: { theme?: Theme; width?: number; fullPage?: boolean } = {},
): Promise<string> {
  const theme = opts.theme ?? "light";
  const width = opts.width ?? page.viewportSize()?.width ?? 1440;
  const dir = path.join(SPEC184_SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  const file = path.join(dir, `${name}-${theme}-${width}.png`);
  await page.screenshot({
    path: file,
    fullPage: opts.fullPage ?? false,
    animations: "disabled",
    caret: "hide",
    timeout: 15_000,
  });
  return file;
}
