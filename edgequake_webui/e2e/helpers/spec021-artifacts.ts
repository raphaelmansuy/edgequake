/**
 * SPEC-021 artifact paths — single source for P5-01 zero-documents fix proof.
 */
import fs from "node:fs";
import path from "node:path";

export const SPEC021_ROOT = path.resolve(__dirname, "../../../specs/021-storage-study");
export const SPEC021_SCREENSHOTS = path.join(SPEC021_ROOT, "e2e/screenshots");

export function ensureSpec021ScreenshotDir(): void {
  fs.mkdirSync(SPEC021_SCREENSHOTS, { recursive: true });
}

export function spec021ScreenshotPath(name: string): string {
  ensureSpec021ScreenshotDir();
  return path.join(SPEC021_SCREENSHOTS, name);
}
