/**
 * Canonical screenshot output paths for Playwright E2E.
 *
 * Never write intentional captures to `test-results/` (Playwright failure artifacts only).
 *
 * Layout:
 *   audit_ui/screenshots/{subdir}/     — formal UX audit (verification, screens, components, …)
 *   edgequake_webui/e2e/screenshots/{subdir}/ — spec-local captures (citations, debug, load, …)
 */
import fs from "node:fs";
import path from "node:path";

/** Repository root (`edgequake/`) */
export const REPO_ROOT = path.resolve(__dirname, "../../..");

/** WebUI package root (`edgequake_webui/`) */
export const WEBUI_ROOT = path.resolve(__dirname, "../..");

export const SCREENSHOT_ROOT = {
  audit: path.join(REPO_ROOT, "audit_ui/screenshots"),
  e2e: path.join(WEBUI_ROOT, "e2e/screenshots"),
} as const;

export type AuditScreenshotSubdir =
  | "verification"
  | "screens"
  | "components"
  | "states"
  | "responsive"
  | "scroll"
  | "themes"
  | "panels"
  | "accessibility";

export type E2eScreenshotSubdir =
  | "audit-verification"
  | "ingestion"
  | "ingestion-interactive"
  | "citations"
  | "streaming"
  | "query"
  | "chat"
  | "debug"
  | "load"
  | "crawl"
  | "issues";

export function ensureDir(dir: string): string {
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

/** `audit_ui/screenshots/<subdir>/` (created if missing) */
export function auditScreenshotDir(subdir?: string): string {
  return ensureDir(
    subdir
      ? path.join(SCREENSHOT_ROOT.audit, subdir)
      : SCREENSHOT_ROOT.audit,
  );
}

/** `audit_ui/screenshots/<subdir>/<fileName>` — omit subdir for audit root */
export function auditScreenshot(
  subdir: AuditScreenshotSubdir | string,
  fileName: string,
): string {
  return path.join(auditScreenshotDir(subdir), fileName);
}

/** Audit capture with optional nested subdir (e.g. `screens/dashboard`). */
export function resolveAuditPath(
  subdir: string | undefined,
  fileName: string,
): string {
  return path.join(auditScreenshotDir(subdir), fileName);
}

/** `e2e/screenshots/<subdir>/<fileName>` */
export function e2eScreenshot(
  subdir: E2eScreenshotSubdir | string,
  fileName: string,
): string {
  return path.join(ensureDir(path.join(SCREENSHOT_ROOT.e2e, subdir)), fileName);
}

/** `e2e/screenshots/issues/<issueId>/<fileName>` */
export function issueScreenshot(issueId: string, fileName: string): string {
  return e2eScreenshot(`issues/${issueId}`, fileName);
}
