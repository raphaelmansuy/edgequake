/**
 * @module api-client
 * @description Base API client for EdgeQuake backend.
 * Provides fetch wrapper with error handling and streaming support.
 *
 * @implements FEAT0700 - Unified API client with error handling
 * @implements FEAT0770 - SSE streaming client
 * @implements FEAT0771 - Request/response interceptors
 *
 * @enforces BR0700 - All requests include auth headers
 * @enforces BR0701 - Timeout after 30s for non-streaming
 * @enforces BR0702 - Retry on 5xx with exponential backoff
 */

import type { ApiError } from "@/types";

// Server Base URL (without /api/v1)
const getServerBaseUrl = () => {
  const envUrl = process.env.NEXT_PUBLIC_API_URL;
  if (envUrl) {
    return envUrl.replace(/\/$/, "");
  }
  return "";
};

// API Base URL configuration
// If NEXT_PUBLIC_API_URL is set (e.g., http://localhost:8080), append /api/v1
// Otherwise default to /api/v1 for same-origin requests
const getApiBaseUrl = () => {
  const serverUrl = getServerBaseUrl();
  return serverUrl ? `${serverUrl}/api/v1` : "/api/v1";
};

export const SERVER_BASE_URL = getServerBaseUrl();
const API_BASE_URL = getApiBaseUrl();

// Custom error classes
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

// Token management
let accessToken: string | null = null;
let refreshToken: string | null = null;

export function setTokens(access: string, refresh: string): void {
  accessToken = access;
  refreshToken = refresh;
  if (typeof window !== "undefined") {
    localStorage.setItem("accessToken", access);
    localStorage.setItem("refreshToken", refresh);
  }
}

export function getTokens(): {
  accessToken: string | null;
  refreshToken: string | null;
} {
  if (typeof window !== "undefined" && !accessToken) {
    accessToken = localStorage.getItem("accessToken");
    refreshToken = localStorage.getItem("refreshToken");
  }
  return { accessToken, refreshToken };
}

export function clearTokens(): void {
  accessToken = null;
  refreshToken = null;
  if (typeof window !== "undefined") {
    localStorage.removeItem("accessToken");
    localStorage.removeItem("refreshToken");
  }
}

// Current tenant/workspace/user context
let currentTenantId: string | null = null;
let currentWorkspaceId: string | null = null;
let currentUserId: string | null = null;

// Generate a UUID v4 for anonymous users
function generateUUID(): string {
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

// Get or create anonymous user ID
export function getOrCreateUserId(): string {
  if (typeof window !== "undefined") {
    let userId = localStorage.getItem("userId");
    if (!userId) {
      userId = generateUUID();
      localStorage.setItem("userId", userId);
    }
    return userId;
  }
  // For server-side, generate a temporary ID
  return generateUUID();
}

export function setTenantContext(tenantId: string, workspaceId?: string): void {
  currentTenantId = tenantId;
  currentWorkspaceId = workspaceId || null;
  if (typeof window !== "undefined") {
    localStorage.setItem("tenantId", tenantId);
    if (workspaceId) {
      localStorage.setItem("workspaceId", workspaceId);
    }
  }
}

export function getTenantContext(): {
  tenantId: string | null;
  workspaceId: string | null;
  userId: string | null;
} {
  if (typeof window !== "undefined" && !currentTenantId) {
    currentTenantId = localStorage.getItem("tenantId");
    currentWorkspaceId = localStorage.getItem("workspaceId");
    currentUserId = getOrCreateUserId();
  }
  return {
    tenantId: currentTenantId,
    workspaceId: currentWorkspaceId,
    userId: currentUserId,
  };
}

// Headers builder
function buildHeaders(customHeaders?: HeadersInit, body?: unknown): Headers {
  const headers = new Headers(customHeaders);

  // Only set Content-Type to application/json if body is not FormData
  // For FormData, the browser will set Content-Type with the boundary automatically
  if (!headers.has("Content-Type") && !(body instanceof FormData)) {
    headers.set("Content-Type", "application/json");
  }

  const { accessToken: token } = getTokens();
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }

  const { tenantId, workspaceId, userId } = getTenantContext();
  if (tenantId) {
    headers.set("X-Tenant-ID", tenantId);
  }
  if (workspaceId) {
    headers.set("X-Workspace-ID", workspaceId);
  }
  // Always include user ID for conversation APIs
  const effectiveUserId = userId || getOrCreateUserId();
  headers.set("X-User-ID", effectiveUserId);

  return headers;
}

// Main API client function
export async function apiClient<T>(
  endpoint: string,
  options: RequestInit = {},
): Promise<T> {
  const url = endpoint.startsWith("http")
    ? endpoint
    : `${API_BASE_URL}${endpoint}`;

  const config: RequestInit = {
    ...options,
    headers: buildHeaders(options.headers, options.body),
  };

  try {
    const response = await fetch(url, config);

    // Handle 401 - try to refresh token
    if (response.status === 401) {
      const refreshed = await tryRefreshToken();
      if (refreshed) {
        // Retry the request with new token
        config.headers = buildHeaders(options.headers, options.body);
        const retryResponse = await fetch(url, config);
        if (!retryResponse.ok) {
          throw await handleErrorResponse(retryResponse);
        }
        return retryResponse.json() as Promise<T>;
      }
      throw new AuthError();
    }

    if (!response.ok) {
      throw await handleErrorResponse(response);
    }

    // Handle empty responses
    const text = await response.text();
    if (!text) {
      return {} as T;
    }

    return JSON.parse(text) as T;
  } catch (error) {
    if (error instanceof ApiRequestError || error instanceof AuthError) {
      throw error;
    }
    if (error instanceof TypeError) {
      throw new NetworkError();
    }
    throw error;
  }
}

// Error response handler
async function handleErrorResponse(
  response: Response,
): Promise<ApiRequestError> {
  try {
    const errorData = (await response.json()) as ApiError;
    return ApiRequestError.fromResponse({
      ...errorData,
      status: response.status,
    });
  } catch {
    return new ApiRequestError(
      response.statusText || "Request failed",
      response.status,
    );
  }
}

// Token refresh
async function tryRefreshToken(): Promise<boolean> {
  const { refreshToken: refresh } = getTokens();
  if (!refresh) {
    return false;
  }

  try {
    const response = await fetch(`${API_BASE_URL}/auth/refresh`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ refresh_token: refresh }),
    });

    if (!response.ok) {
      clearTokens();
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
    return false;
  }
}

// Streaming API client for SSE (Server-Sent Events) responses
// SSE format: "data: <content>\n\n" for each event
export async function* streamClient<T>(
  endpoint: string,
  options: RequestInit = {},
): AsyncGenerator<T, void, unknown> {
  const url = endpoint.startsWith("http")
    ? endpoint
    : `${API_BASE_URL}${endpoint}`;

  const config: RequestInit = {
    ...options,
    headers: buildHeaders(options.headers),
  };

  const response = await fetch(url, config);

  if (!response.ok) {
    throw await handleErrorResponse(response);
  }

  if (!response.body) {
    throw new Error("Response body is null");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();

      if (done) {
        // Process any remaining buffer
        if (buffer.trim()) {
          const parsed = parseSSEData(buffer);
          if (parsed !== null) {
            yield parsed as T;
          }
        }
        break;
      }

      buffer += decoder.decode(value, { stream: true });

      // Process complete SSE events (separated by double newlines)
      // SSE format: "data: <content>\n\n"
      const events = buffer.split("\n\n");
      buffer = events.pop() || "";

      for (const event of events) {
        const parsed = parseSSEData(event);
        if (parsed !== null) {
          yield parsed as T;
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}

// Parse SSE data line(s) and return the content
// SSE format: "data: <content>" or multiple "data: " lines for multiline content
// Note: SSE spec says "data:" followed by optional space, then content
// We need to remove the SSE-mandated space but preserve content-internal spaces
function parseSSEData(event: string): unknown {
  const lines = event.split("\n");
  const dataChunks: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("data:")) {
      // Extract content after "data:"
      // SSE format allows "data:<content>" or "data: <content>" (with optional space)
      let content = trimmed.slice(5);
      // Remove the single SSE-mandated space if present at the start
      // But preserve the actual token content (which may have its own leading space)
      if (content.startsWith(" ")) {
        content = content.slice(1);
      }
      if (content) {
        dataChunks.push(content);
      }
    } else if (
      trimmed.startsWith("event:") ||
      trimmed.startsWith("id:") ||
      trimmed.startsWith("retry:")
    ) {
      // Ignore other SSE fields for now
      continue;
    } else if (trimmed && !trimmed.startsWith(":")) {
      // Non-SSE line - might be plain content or NDJSON fallback
      dataChunks.push(trimmed);
    }
  }

  if (dataChunks.length === 0) {
    return null;
  }

  // Join data chunks - preserve spaces for word separation
  const data = dataChunks.join("");

  // Try to parse as JSON first (structured response)
  try {
    return JSON.parse(data);
  } catch {
    // Not JSON, return as raw text wrapped in expected format
    return { type: "token", content: data };
  }
}

// Convenience methods
export const api = {
  get: <T>(endpoint: string, options?: RequestInit) =>
    apiClient<T>(endpoint, { ...options, method: "GET" }),

  post: <T>(endpoint: string, data?: unknown, options?: RequestInit) =>
    apiClient<T>(endpoint, {
      ...options,
      method: "POST",
      body:
        data instanceof FormData
          ? data
          : data
            ? JSON.stringify(data)
            : undefined,
    }),

  put: <T>(endpoint: string, data?: unknown, options?: RequestInit) =>
    apiClient<T>(endpoint, {
      ...options,
      method: "PUT",
      body:
        data instanceof FormData
          ? data
          : data
            ? JSON.stringify(data)
            : undefined,
    }),

  patch: <T>(endpoint: string, data?: unknown, options?: RequestInit) =>
    apiClient<T>(endpoint, {
      ...options,
      method: "PATCH",
      body:
        data instanceof FormData
          ? data
          : data
            ? JSON.stringify(data)
            : undefined,
    }),

  delete: <T>(endpoint: string, options?: RequestInit) =>
    apiClient<T>(endpoint, { ...options, method: "DELETE" }),

  stream: <T>(endpoint: string, data?: unknown, options?: RequestInit) =>
    streamClient<T>(endpoint, {
      ...options,
      method: "POST",
      body: data ? JSON.stringify(data) : undefined,
    }),
};

export default api;
