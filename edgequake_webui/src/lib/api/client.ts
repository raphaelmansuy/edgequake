/**
 * @module api-client
 * @description Base API client for EdgeQuake backend.
 *
 * Provides a fetch wrapper with error handling, auth-token refresh, and
 * unified logging. Session context (tokens, tenant IDs, traceparent) lives
 * in `client-context.ts`; SSE streaming lives in `stream-client.ts`; the
 * backend readiness probe lives in `backend-readiness.ts`. This module owns
 * only the request/response transport.
 *
 * @implements FEAT0700 - Unified API client with error handling
 * @implements FEAT0771 - Request/response interceptors
 *
 * @enforces BR0700 - All requests include auth headers
 * @enforces BR0701 - Timeout after 30s for non-streaming
 * @enforces BR0702 - Retry on 5xx with exponential backoff
 */

import { getRuntimeApiBaseUrl, getRuntimeServerBaseUrl } from "@/lib/runtime-config";
import {
  applyRestoredAccessSession,
  type RestoredAccessSession,
} from "@/stores/use-auth-store";
import type { ApiError } from "@/types";
import {
  adoptTraceparentFromResponse,
  clearTokens,
  generateRequestId,
  generateTraceparent,
  getClientTraceContext,
  getOrCreateUserId,
  getTenantContext,
  getTokens,
} from "./client-context";
import { streamClient } from "./stream-client";

export type { RestoredAccessSession };

export const SERVER_BASE_URL = getRuntimeServerBaseUrl();

// Re-export session context so existing `import { ... } from "./client"` keeps working.
export {
  setTokens,
  getTokens,
  clearTokens,
  getOrCreateUserId,
  setTenantContext,
  getTenantContext,
  generateTraceparent,
  getClientTraceContext,
  adoptTraceparentFromResponse,
} from "./client-context";

// Re-export streaming + readiness (SRP modules).
export { streamClient } from "./stream-client";
export {
  isBackendReady,
  getBackendReadinessState,
  getBackendReadinessSnapshot,
  probeBackendReadiness,
  probeBackendReadinessSnapshot,
  _resetBackendReadinessCache,
  type ReadinessProbeClient,
  type BackendReadinessState,
  type BackendReadinessSnapshot,
} from "./backend-readiness";

// === Custom error classes =============================================

export class ApiRequestError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string,
    public details?: Record<string, unknown>,
  ) {
    super(message);
    this.name = "ApiRequestError";
  }

  static fromResponse(error: ApiError): ApiRequestError {
    return new ApiRequestError(
      error.message,
      error.status,
      error.code,
      error.details,
    );
  }
}

export class AuthError extends ApiRequestError {
  constructor(message: string = "Authentication required") {
    super(message, 401, "AUTH_REQUIRED");
    this.name = "AuthError";
  }
}

export class NetworkError extends Error {
  constructor(message: string = "Network request failed") {
    super(message);
    this.name = "NetworkError";
  }
}

// === Shared options + logging =========================================

export interface ClientErrorLogOptions {
  silent?: boolean;
}

/** Default non-streaming request timeout (BR0701). Documents override lower. */
export const DEFAULT_API_TIMEOUT_MS = 30_000;

/** Interactive document list/detail timeout — fail before browser TCP hang. */
export const DOCUMENTS_API_TIMEOUT_MS = 10_000;

export interface ApiClientOptions extends RequestInit {
  /** Client-only guard for an operation whose account/workspace intent is frozen. */
  assertScope?: () => void;
  /**
   * Suppress console.error on transport failures (default: false).
   *
   * WHY: Next.js dev overlay promotes any console.error to a full-screen
   * error modal. Health probes and background polls must use silent:true so
   * a not-yet-ready backend does not crash the dashboard on boot.
   *
   * Non-silent calls (user-initiated mutations, page loads) still log via
   * console.warn — visible in devtools but never promoted to the overlay.
   */
  silent?: boolean;
  /**
   * Abort the request after this many ms (default: {@link DEFAULT_API_TIMEOUT_MS}).
   * Pass `0` to disable. Combined with any caller-provided `signal` via AbortSignal.any.
   */
  timeoutMs?: number;
  /** Do not send X-Tenant-ID. Fleet provider creates are deployment-scoped. */
  omitTenant?: boolean;
}

/** Request aborted because the client-side deadline elapsed. */
export class TimeoutError extends ApiRequestError {
  constructor(message = "Request timed out") {
    super(message, 504, "timeout");
    this.name = "TimeoutError";
  }
}

function isAbortTimeoutError(error: unknown): boolean {
  // AbortSignal.timeout() rejects with DOMException name "TimeoutError".
  // User/navigation AbortError is left alone (not remapped to our TimeoutError).
  return (error as { name?: string })?.name === "TimeoutError";
}

function resolveFetchSignal(
  timeoutMs: number | undefined,
  userSignal: AbortSignal | null | undefined,
): AbortSignal | undefined {
  const effectiveTimeout =
    timeoutMs === undefined ? DEFAULT_API_TIMEOUT_MS : timeoutMs;
  const parts: AbortSignal[] = [];
  if (effectiveTimeout > 0) {
    parts.push(AbortSignal.timeout(effectiveTimeout));
  }
  if (userSignal) {
    parts.push(userSignal);
  }
  if (parts.length === 0) return undefined;
  if (parts.length === 1) return parts[0];
  return AbortSignal.any(parts);
}

/** Structured payload for client-side error logging (explicit context). */
export function apiErrorLogPayload(
  err: ApiRequestError,
  context: { url?: string } = {},
): Record<string, unknown> {
  const trace = getClientTraceContext();
  const payload: Record<string, unknown> = {
    status: err.status,
    code: err.code ?? null,
    message: err.message || "(no message)",
    retryable: err.details?.retryable ?? null,
    source: err.details?.source ?? null,
    diagnostics: err.details?.diagnostics ?? null,
    request_id: err.details?.request_id ?? null,
    traceparent: trace.traceparent ?? null,
    trace_id: trace.trace_id ?? null,
  };
  if (context.url) {
    payload.url = context.url;
  }
  return payload;
}

/** Log API errors at the correct level with full backend context. */
export function logClientApiError(
  err: ApiRequestError,
  options: ClientErrorLogOptions & { url?: string } = {},
): void {
  const payload = apiErrorLogPayload(err, { url: options.url });
  if (options.silent) {
    console.debug("[edgequake] API probe failed", payload);
    return;
  }
  // WHY console.warn (not console.error): Next.js dev promotes console.error
  // to a full-screen overlay. React Query and hooks handle failures via
  // isError/toast — logging must stay visible in devtools without hijacking
  // the page (same policy as network errors and 4xx responses).
  if (err.status >= 500) {
    console.warn("[edgequake] API server error", payload);
  } else if (err.status >= 400) {
    console.warn("[edgequake] API client error", payload);
  }
}

/** Log transport failures (no HTTP response) with trace context.
 *
 * WHY console.warn (not console.error): Next.js dev promotes console.error
 * to a full-screen error overlay, which crashes the dashboard whenever the
 * backend is briefly unavailable (cold start, restart, network blip). The
 * underlying failure is still thrown to callers — they decide UX via
 * React Query's `isError` / retry policies. console.warn keeps the failure
 * visible in devtools without hijacking the page.
 */
export function logClientNetworkError(
  err: NetworkError,
  options: ClientErrorLogOptions = {},
): void {
  const trace = getClientTraceContext();
  const payload = {
    message: err.message,
    code: "NETWORK_ERROR",
    source: "webui_client",
    apiBase: getRuntimeApiBaseUrl() || "(relative via dev proxy)",
    traceparent: trace.traceparent,
    trace_id: trace.trace_id,
  };
  if (options.silent) {
    console.debug("[edgequake] Network probe failed", payload);
    return;
  }
  console.warn("[edgequake] Network error", payload);
}

// === Headers + URL resolution =========================================

/** Build the request headers: Content-Type, Authorization, X-Request-ID,
 * traceparent, and tenant/workspace/user context. */
export function buildHeaders(customHeaders?: HeadersInit, body?: unknown): Headers {
  const headers = new Headers(customHeaders);

  // For FormData, the browser sets Content-Type with the boundary automatically.
  if (!headers.has("Content-Type") && !(body instanceof FormData)) {
    headers.set("Content-Type", "application/json");
  }

  const { accessToken: token } = getTokens();
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  // SPEC-018: Per-request correlation for distributed tracing / support tickets
  if (!headers.has("X-Request-ID")) {
    headers.set(
      "X-Request-ID",
      typeof crypto !== "undefined" && crypto.randomUUID
        ? crypto.randomUUID()
        : generateRequestId(),
    );
  }

  if (!headers.has("traceparent")) {
    headers.set("traceparent", generateTraceparent());
  }

  const { tenantId, workspaceId, userId } = getTenantContext();
  const omitTenant = headers.get("X-Edgequake-Omit-Tenant") === "1";
  if (omitTenant) headers.delete("X-Edgequake-Omit-Tenant");
  // Allow callers to override tenant/workspace (e.g. create-tenant wizard
  // updating the new tenant's Default Workspace before store selection).
  if (tenantId && !omitTenant && !headers.has("X-Tenant-ID")) {
    headers.set("X-Tenant-ID", tenantId);
  }
  if (workspaceId && !headers.has("X-Workspace-ID")) {
    headers.set("X-Workspace-ID", workspaceId);
  }
  headers.set("X-User-ID", userId || getOrCreateUserId());
  // SPEC-154: identify SPA so login/refresh omit refresh_token JSON (cookie SSOT).
  if (!headers.has("X-Edgequake-Client")) {
    headers.set("X-Edgequake-Client", "webui");
  }

  return headers;
}

/** Resolve URL for backend root endpoints (/health, /ready) — not under /api/v1. */
export function resolveServerRootUrl(endpoint: string): string {
  if (endpoint.startsWith("http")) return endpoint;
  const path = endpoint.startsWith("/") ? endpoint : `/${endpoint}`;
  const serverBaseUrl = getRuntimeServerBaseUrl();
  return serverBaseUrl ? `${serverBaseUrl}${path}` : path;
}

// === Error response normalization =====================================

/** Normalize a non-2xx response into an `ApiRequestError` with diagnostics. */
export async function handleErrorResponse(
  response: Response,
  options: ClientErrorLogOptions = {},
): Promise<ApiRequestError> {
  const requestId = response.headers.get("x-request-id") ?? undefined;
  try {
    const errorData = (await response.json()) as ApiError & {
      details?: Record<string, unknown> & { request_id?: string };
    };
    const err = ApiRequestError.fromResponse({
      ...errorData,
      status: response.status,
    });
    err.details = {
      ...(typeof errorData.details === "object" ? errorData.details : {}),
      ...err.details,
      request_id:
        requestId ??
        (typeof errorData.details?.request_id === "string"
          ? errorData.details.request_id
          : undefined),
    };
    logClientApiError(err, { ...options, url: response.url });
    return err;
  } catch {
    const err = new ApiRequestError(
      response.statusText || "Request failed",
      response.status,
    );
    if (requestId) err.details = { request_id: requestId };
    logClientApiError(err, { ...options, url: response.url });
    return err;
  }
}

// === Server-root probe client (no auth) ===============================

export interface ServerRootClientOptions extends RequestInit {
  /** Probe failures (health/ready) are handled by callers — avoid
   * console.error in dev, which triggers the Next.js global error overlay. */
  silent?: boolean;
}

/** GET/POST to backend server root (health, readiness). No auth headers —
 * probes must work before login. @implements UI-DRY-003 */
export async function serverRootClient<T>(
  endpoint: string,
  options: ServerRootClientOptions = {},
): Promise<T> {
  const { silent = false, ...fetchOptions } = options;
  const url = resolveServerRootUrl(endpoint);

  try {
    const response = await fetch(url, {
      ...fetchOptions,
      method: fetchOptions.method ?? "GET",
    });

    if (!response.ok) {
      throw await handleErrorResponse(response, { silent });
    }

    const text = await response.text();
    return text ? (JSON.parse(text) as T) : ({} as T);
  } catch (error) {
    if (error instanceof ApiRequestError) throw error;
    if (error instanceof TypeError) {
      const networkErr = new NetworkError();
      logClientNetworkError(networkErr, { silent });
      throw networkErr;
    }
    throw error;
  }
}

// === Main API client ==================================================

/** Main API client function. */
export async function apiClient<T>(
  endpoint: string,
  options: ApiClientOptions = {},
): Promise<T> {
  const { silent = false, timeoutMs, signal: userSignal, omitTenant = false, assertScope, ...fetchOptions } =
    options;
  assertScope?.();
  const url = endpoint.startsWith("http")
    ? endpoint
    : `${getRuntimeApiBaseUrl()}${endpoint}`;

  const callerHeaders = new Headers(fetchOptions.headers);
  if (omitTenant) callerHeaders.set("X-Edgequake-Omit-Tenant", "1");
  const signal = resolveFetchSignal(timeoutMs, userSignal);
  const config: RequestInit = {
    ...fetchOptions,
    credentials: fetchOptions.credentials ?? "include",
    signal,
    headers: buildHeaders(callerHeaders, fetchOptions.body),
  };

  try {
    const response = await fetch(url, config);

    // Handle 401 - try to refresh token
    if (response.status === 401) {
      const refreshed = await tryRefreshToken();
      if (refreshed) {
        assertScope?.();
        config.headers = buildHeaders(callerHeaders, fetchOptions.body);
        // Fresh deadline for the retry attempt.
        config.signal = resolveFetchSignal(timeoutMs, userSignal);
        const retryResponse = await fetch(url, config);
        if (!retryResponse.ok) {
          throw await handleErrorResponse(retryResponse, { silent });
        }
        return retryResponse.json() as Promise<T>;
      }
      throw new AuthError();
    }

    if (!response.ok) {
      adoptTraceparentFromResponse(response);
      throw await handleErrorResponse(response, { silent });
    }

    adoptTraceparentFromResponse(response);

    const text = await response.text();
    return text ? (JSON.parse(text) as T) : ({} as T);
  } catch (error) {
    if (error instanceof ApiRequestError || error instanceof AuthError) {
      throw error;
    }
    if (isAbortTimeoutError(error)) {
      const timeoutErr = new TimeoutError();
      logClientApiError(timeoutErr, { silent, url });
      throw timeoutErr;
    }
    if (error instanceof TypeError) {
      const networkErr = new NetworkError();
      logClientNetworkError(networkErr, { silent });
      throw networkErr;
    }
    throw error;
  }
}

/**
 * Single-flight slot for refresh (rotate-on-use — concurrent POSTs cause reuse logout).
 * Cleared in `finally` so a failure never sticks.
 */
let refreshInFlight: Promise<RestoredAccessSession | null> | null = null;

async function redeemRefreshCookieOnce(): Promise<RestoredAccessSession | null> {
  try {
    const response = await fetch(`${getRuntimeApiBaseUrl()}/auth/refresh`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "include",
      body: JSON.stringify({}),
    });

    if (!response.ok) {
      return null;
    }

    const data = (await response.json()) as {
      access_token?: string;
      refresh_token?: string;
      expires_in?: number;
    };
    if (!data.access_token) {
      return null;
    }
    const expires_in =
      typeof data.expires_in === "number" && data.expires_in > 0
        ? data.expires_in
        : 900;
    const session: RestoredAccessSession = {
      access_token: data.access_token,
      expires_in,
    };
    applyRestoredAccessSession(session);
    return session;
  } catch {
    return null;
  }
}

/**
 * Redeem `eq_refresh` (hard refresh / soft-expiry / 401 retry).
 *
 * Coalesces concurrent callers onto one network round-trip. Does not clear
 * tokens or dispatch logout — callers decide. AuthGuard uses this before
 * redirecting to `/login`; the 401 retry path clears on failure.
 */
export async function restoreSessionFromRefreshCookie(): Promise<RestoredAccessSession | null> {
  if (!refreshInFlight) {
    refreshInFlight = redeemRefreshCookieOnce().finally(() => {
      refreshInFlight = null;
    });
  }
  return refreshInFlight;
}

/**
 * Shared refresh for apiClient + multipart (SPEC-154).
 * On failure: clear memory tokens and notify AuthGuard.
 */
export async function refreshAccessToken(): Promise<boolean> {
  const restored = await restoreSessionFromRefreshCookie();
  if (!restored) {
    clearTokens();
    dispatchAuthFailure();
    return false;
  }
  return true;
}

/** Token refresh — used by the main client on 401 (SPEC-154: HttpOnly cookie). */
async function tryRefreshToken(): Promise<boolean> {
  return refreshAccessToken();
}

/** Dispatch a browser event so AuthGuard can react to permanent auth failures. */
export function dispatchAuthFailure(): void {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new Event("auth:logout-required"));
  }
}

// === Convenience methods ==============================================

export const api = {
  get: <T>(endpoint: string, options?: ApiClientOptions) =>
    apiClient<T>(endpoint, { ...options, method: "GET" }),

  post: <T>(endpoint: string, data?: unknown, options?: ApiClientOptions) =>
    apiClient<T>(endpoint, {
      ...options,
      method: "POST",
      body: data instanceof FormData ? data : data ? JSON.stringify(data) : undefined,
    }),

  put: <T>(endpoint: string, data?: unknown, options?: ApiClientOptions) =>
    apiClient<T>(endpoint, {
      ...options,
      method: "PUT",
      body: data instanceof FormData ? data : data ? JSON.stringify(data) : undefined,
    }),

  patch: <T>(endpoint: string, data?: unknown, options?: ApiClientOptions) =>
    apiClient<T>(endpoint, {
      ...options,
      method: "PATCH",
      body: data instanceof FormData ? data : data ? JSON.stringify(data) : undefined,
    }),

  delete: <T>(endpoint: string, options?: ApiClientOptions) =>
    apiClient<T>(endpoint, { ...options, method: "DELETE" }),

  stream: <T>(endpoint: string, data?: unknown, options?: RequestInit) =>
    streamClient<T>(endpoint, {
      ...options,
      method: "POST",
      body: data ? JSON.stringify(data) : undefined,
    }),

  serverRoot: {
    get: <T>(endpoint: string, options?: RequestInit) =>
      serverRootClient<T>(endpoint, { ...options, method: "GET" }),
  },
};

export default api;
