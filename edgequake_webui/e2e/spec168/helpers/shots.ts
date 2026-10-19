/**
 * SPEC-168 screenshot helper — visual inspection loop.
 * Writes under specs/168-perfect-ui/e2e/screenshots/{round}/
 */
import fs from "node:fs";
import path from "node:path";
import type { Page } from "@playwright/test";

const REPO_ROOT = path.resolve(__dirname, "../../../../");
export const SPEC168_SHOTS_ROOT = path.join(
  REPO_ROOT,
  "specs/168-perfect-ui/e2e/screenshots",
);

export type Theme = "light" | "dark";

export const ROUND = process.env.SPEC168_ROUND ?? "round-00-baseline";

export function shotFile(name: string, theme: Theme, width: number): string {
  const dir = path.join(SPEC168_SHOTS_ROOT, ROUND);
  fs.mkdirSync(dir, { recursive: true });
  return path.join(dir, `${name}-${theme}-${width}.png`);
}

export async function applyTheme(page: Page, theme: Theme): Promise<void> {
  await page.evaluate((t) => {
    const root = document.documentElement;
    root.classList.toggle("dark", t === "dark");
    localStorage.setItem("theme", t);
  }, theme);
}

export async function setViewport(page: Page, width: number): Promise<void> {
  const height = width <= 480 ? 812 : width <= 900 ? 1024 : 800;
  await page.setViewportSize({ width, height });
}

export async function shot(
  page: Page,
  name: string,
  theme: Theme,
  width: number,
  opts: { fullPage?: boolean } = {},
): Promise<string> {
  const file = shotFile(name, theme, width);
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
    // Headless Chrome GPU compositing can stall a frame; one settle + retry.
    await page.waitForTimeout(500);
    await take();
  }
  return file;
}
