/**
 * @module backend-readiness
 * @description Lightweight backend readiness probe for SPEC-021 stabilization.
 *
 * Extracted from `client.ts` to keep that module under the SPEC-017 LOC cap
 * (SRP: this module owns only "is the backend up?" — transport + caching).
 *
 * WHY: The dashboard must degrade gracefully when the backend is not yet
 * ready (cold start, rolling restart, transient DNS failure). React Query
 * retries transport errors silently; this probe gives the UI a single
 * cached source of truth for the "Backend not reachable" banner.
 */

import { getRuntimeServerBaseUrl } from "@/lib/runtime-config";

export interface ReadinessProbeResponse {
  /** Whether the response status was 2xx. */
  ok: () => boolean;
  /** HTTP status code (used to distinguish 401/403 "reachable but auth-gated"
   * from a true transport failure). */
  status: () => number;
  /** Parse the response body as JSON. */
  json: () => Promise<unknown>;
}

export interface ReadinessProbeClient {
  /** Issue a GET request and return a Playwright-style response shim. */
  get: (url: string) => Promise<ReadinessProbeResponse>;
}

// Cache the last backend readiness result to avoid repeated probes during boot.
let backendReadinessCache: { ready: boolean; checkedAt: number } | null = null;
const READINESS_CACHE_MS = 5_000;

/**
 * Check if the backend is ready to serve requests.
 *
 * Returns a cached result within {@link READINESS_CACHE_MS} to avoid stampeding
 * the health endpoint on dashboard boot when multiple React Query hooks fire
 * simultaneously.
 *
 * @param request Probe client (defaults to a native fetch adapter with a 2s
 *   timeout). Tests can pass a Playwright `APIRequestContext`-compatible shim.
 */
export async function isBackendReady(
  request: ReadinessProbeClient = nativeFetchAdapter(),
): Promise<boolean> {
  const now = Date.now();
  if (
    backendReadinessCache &&
    now - backendReadinessCache.checkedAt < READINESS_CACHE_MS
  ) {
    return backendReadinessCache.ready;
  }

  try {
    const res = await request.get(`${getRuntimeServerBaseUrl()}/health`);
    // WHY treat 401/403 as reachable: some deployments put /health behind auth.
    // The backend is up and responding — the banner should not fire. A true
    // unreachable backend throws (TypeError) which is caught below.
    if (res.ok()) {
      const body = (await res.json()) as { status?: string };
      const ready = body.status === "healthy" || body.status === "degraded";
      backendReadinessCache = { ready, checkedAt: now };
      return ready;
    }
    const status = res.status();
    if (status === 401 || status === 403) {
      // WHY: some deployments put /health behind auth. The backend is up and
      // responding — the banner should not fire. A true unreachable backend
      // throws (TypeError) which is caught below.
      backendReadinessCache = { ready: true, checkedAt: now };
      return true;
    }
    backendReadinessCache = { ready: false, checkedAt: now };
    return false;
  } catch {
    backendReadinessCache = { ready: false, checkedAt: now };
    return false;
  }
}

/** Reset the cached readiness result (used by tests). */
export function _resetBackendReadinessCache(): void {
  backendReadinessCache = null;
}

function nativeFetchAdapter(): ReadinessProbeClient {
  return {
    get: async (url: string) => {
      const res = await fetch(url, { signal: AbortSignal.timeout(2_000) });
      return {
        ok: () => res.ok,
        status: () => res.status,
        json: () => res.json(),
      };
    },
  };
}
