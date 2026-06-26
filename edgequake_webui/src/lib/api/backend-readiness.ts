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
 *
 * SPEC-021 P-G13: **liveness ≠ deep health**. Under heavy ingestion the DB
 * pool may be saturated; `/health` storage pings can exceed the old 2s probe
 * budget even though the process is alive (`/live` → "OK"). The banner must
 * distinguish *unreachable* (process down) from *degraded* (busy but serving).
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
  /** Plain-text body (for `/live` → "OK"). */
  text: () => Promise<string>;
}

export interface ReadinessProbeClient {
  /** Issue a GET request and return a Playwright-style response shim. */
  get: (url: string) => Promise<ReadinessProbeResponse>;
}

/** Reachability state for the dashboard banner (P-G13). */
export type BackendReadinessState =
  | "ready"
  | "degraded"
  | "unreachable"
  | "misconfigured";

interface ReadinessCacheState {
  state: BackendReadinessState;
  checkedAt: number;
}

let backendReadinessCache: ReadinessCacheState | null = null;

const SUCCESS_CACHE_MS = 10_000;
const FAILURE_CACHE_MS = 2_000;
const LIVE_PROBE_TIMEOUT_MS = 5_000;
const HEALTH_PROBE_TIMEOUT_MS = 8_000;
const PROBE_ATTEMPTS = 3;
const PROBE_RETRY_DELAY_MS = 300;

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Probe backend readiness with retries and liveness-first semantics.
 *
 * 1. `/live` (no DB) — process alive?
 * 2. `/health` — deep check; timeout/failure → `degraded`, not `unreachable`
 */
export async function probeBackendReadiness(
  request: ReadinessProbeClient = nativeFetchAdapter(),
): Promise<BackendReadinessState> {
  const now = Date.now();
  if (
    backendReadinessCache &&
    now - backendReadinessCache.checkedAt <
      (backendReadinessCache.state === "unreachable"
        ? FAILURE_CACHE_MS
        : SUCCESS_CACHE_MS)
  ) {
    return backendReadinessCache.state;
  }

  const base = getRuntimeServerBaseUrl();
  const liveUrl = `${base}/live`;

  let liveOk = false;
  let misconfigured = false;
  for (let attempt = 0; attempt < PROBE_ATTEMPTS; attempt += 1) {
    try {
      const res = await request.get(liveUrl);
      if (res.ok() && (await res.text()).trim() === "OK") {
        liveOk = true;
        break;
      }
      const status = res.status();
      if (status === 401 || status === 403) {
        misconfigured = true;
        break;
      }
    } catch {
      // retry below
    }
    if (attempt + 1 < PROBE_ATTEMPTS) {
      await sleep(PROBE_RETRY_DELAY_MS * (attempt + 1));
    }
  }

  if (misconfigured) {
    backendReadinessCache = { state: "misconfigured", checkedAt: now };
    return "misconfigured";
  }

  if (!liveOk) {
    backendReadinessCache = { state: "unreachable", checkedAt: now };
    return "unreachable";
  }

  try {
    const res = await request.get(`${base}/health`);
    if (res.ok()) {
      const body = (await res.json()) as { status?: string };
      const state: BackendReadinessState =
        body.status === "healthy" || body.status === "degraded"
          ? "ready"
          : "degraded";
      backendReadinessCache = { state, checkedAt: now };
      return state;
    }
    const status = res.status();
    if (status === 401 || status === 403) {
      backendReadinessCache = { state: "ready", checkedAt: now };
      return "ready";
    }
    backendReadinessCache = { state: "degraded", checkedAt: now };
    return "degraded";
  } catch {
    // Live passed — backend is up but deep health timed out (ingestion load).
    backendReadinessCache = { state: "degraded", checkedAt: now };
    return "degraded";
  }
}

/**
 * Check if the backend is ready to serve requests.
 *
 * Returns true for `ready` and `degraded` — only `unreachable` triggers the
 * "backend not reachable" banner. Degraded gets a softer busy message.
 */
export async function isBackendReady(
  request: ReadinessProbeClient = nativeFetchAdapter(),
): Promise<boolean> {
  const state = await probeBackendReadiness(request);
  return state === "ready" || state === "degraded";
}

/** Expose full state for banners that distinguish busy vs down. */
export async function getBackendReadinessState(
  request: ReadinessProbeClient = nativeFetchAdapter(),
): Promise<BackendReadinessState> {
  return probeBackendReadiness(request);
}

/** Reset the cached readiness result (used by tests). */
export function _resetBackendReadinessCache(): void {
  backendReadinessCache = null;
}

function nativeFetchAdapter(timeoutMs = LIVE_PROBE_TIMEOUT_MS): ReadinessProbeClient {
  return {
    get: async (url: string) => {
      const isHealth = url.endsWith("/health");
      const res = await fetch(url, {
        signal: AbortSignal.timeout(
          isHealth ? HEALTH_PROBE_TIMEOUT_MS : timeoutMs,
        ),
      });
      return {
        ok: () => res.ok,
        status: () => res.status,
        json: () => res.json(),
        text: () => res.text(),
      };
    },
  };
}
