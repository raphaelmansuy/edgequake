/**
 * PDF re-conversion E2E artifact paths.
 */
import fs from "node:fs";
import path from "node:path";

export const RECONVERT_ROOT = path.resolve(
  __dirname,
  "../../../specs/021-storage-study/e2e/screenshots",
);

export function ensureReconvertScreenshotDir(): void {
  fs.mkdirSync(RECONVERT_ROOT, { recursive: true });
}

export function reconvertScreenshotPath(name: string): string {
  ensureReconvertScreenshotDir();
  return path.join(RECONVERT_ROOT, name);
}
