/**
 * Dev-only backend URL resolution for Next.js rewrites.
 *
 * First principle: the browser must proxy to the process that actually serves
 * EdgeQuake `/health`, not whatever happens to listen on :8080 (e.g. another
 * Docker stack). Env vars are hints; we verify with a health probe.
 */

import { execSync } from "node:child_process";
import path from "node:path";

const REPO_ROOT = path.join(__dirname, "..", "..", "..", "..");
const PORT_SCRIPT = path.join(REPO_ROOT, "scripts", "select_edgequake_port.py");

/** Returns true when `baseUrl/health` responds with an EdgeQuake healthy payload. */
export function probeEdgequakeHealth(baseUrl: string): boolean {
  const normalized = baseUrl.replace(/\/$/, "");
  try {
    const body = execSync(`curl -fsS -m 1 "${normalized}/health"`, {
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
    });
    return body.includes('"status"') && body.toLowerCase().includes("healthy");
  } catch {
    return false;
  }
}

/** Discover backend port via shared Makefile port selector (DRY). */
export function discoverBackendUrl(
  preferredPort = 8080,
  scanWindow = 20,
): string {
  const port = execSync(
    `python3 "${PORT_SCRIPT}" backend ${preferredPort} ${scanWindow}`,
    { encoding: "utf-8" },
  ).trim();
  return `http://127.0.0.1:${port}`;
}

/**
 * Resolve the dev proxy target.
 * 1. Validate explicit env URLs against `/health`
 * 2. Auto-discover running EdgeQuake (handles :8080 port collision)
 */
export function resolveDevProxyBackend(): string {
  const candidates = [
    process.env.EDGEQUAKE_API_URL?.trim(),
    process.env.NEXT_PUBLIC_API_URL?.trim(),
  ].filter((value): value is string => Boolean(value));

  for (const raw of candidates) {
    const url = raw.replace(/\/$/, "");
    if (probeEdgequakeHealth(url)) {
      return url;
    }
  }

  const discovered = discoverBackendUrl();
  if (probeEdgequakeHealth(discovered)) {
    return discovered;
  }

  return discovered;
}
