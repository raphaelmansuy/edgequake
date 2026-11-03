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

/**
 * Budget for the dev proxy's own identification probe.
 *
 * WHY not 1 s: this probe decides which port the browser talks to, so a false negative
 * silently redirects the whole UI to a wrong or dead backend and every page 500s — which
 * is exactly what happened when `/health` blocked on a provider reachability check. The
 * server side is now bounded (LLM_PROBE_TIMEOUT), but the probe is kept generous because
 * a cold first request also pays for lazy connection-pool warmup.
 */
const HEALTH_PROBE_TIMEOUT_S = 5;

/** Returns true when `baseUrl/health` responds with an EdgeQuake payload. */
export function probeEdgequakeHealth(baseUrl: string): boolean {
  const normalized = baseUrl.replace(/\/$/, "");
  try {
    const body = execSync(`curl -fsS -m ${HEALTH_PROBE_TIMEOUT_S} "${normalized}/health"`, {
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
    });
    const lower = body.toLowerCase();
    // Accept healthy or degraded — both prove EdgeQuake owns the port (not a foreign :8080 app).
    return (
      body.includes('"status"') &&
      (lower.includes('"status":"healthy"') ||
        lower.includes('"status": "healthy"') ||
        lower.includes('"status":"degraded"') ||
        lower.includes('"status": "degraded"'))
    );
  } catch {
    return false;
  }
}

/** Discover backend port via shared Makefile port selector (DRY). */
export function discoverBackendUrl(
  preferredPort = 8090,
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
