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
  setTokens,
} from "./client-context";
import { streamClient } from "./stream-client";

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
  probeBackendReadiness,
  _resetBackendReadinessCache,
  type ReadinessProbeClient,
  type BackendReadinessState,
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

export interface ApiClientOptions extends RequestInit {
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
  if (tenantId) headers.set("X-Tenant-ID", tenantId);
  if (workspaceId) headers.set("X-Workspace-ID", workspaceId);
  headers.set("X-User-ID", userId || getOrCreateUserId());

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
  const { silent = false, ...fetchOptions } = options;
  const url = endpoint.startsWith("http")
    ? endpoint
    : `${getRuntimeApiBaseUrl()}${endpoint}`;

  const config: RequestInit = {
    ...fetchOptions,
    headers: buildHeaders(fetchOptions.headers, fetchOptions.body),
  };

  try {
    const response = await fetch(url, config);

    // Handle 401 - try to refresh token
    if (response.status === 401) {
      const refreshed = await tryRefreshToken();
      if (refreshed) {
        config.headers = buildHeaders(fetchOptions.headers, fetchOptions.body);
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
    if (error instanceof TypeError) {
      const networkErr = new NetworkError();
      logClientNetworkError(networkErr, { silent });
      throw networkErr;
    }
    throw error;
  }
}

/** Token refresh — used by the main client on 401. */
async function tryRefreshToken(): Promise<boolean> {
  const { refreshToken: refresh } = getTokens();
  if (!refresh) return false;

  try {
    const response = await fetch(`${getRuntimeApiBaseUrl()}/auth/refresh`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ refresh_token: refresh }),
    });

    if (!response.ok) {
      clearTokens();
      dispatchAuthFailure();
      return false;
    }

    const data = (await response.json()) as {
      access_token: string;
      refresh_token: string;
    };
    setTokens(data.access_token, data.refresh_token);
    return true;
  } catch {
    clearTokens();
    dispatchAuthFailure();
    return false;
  }
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
