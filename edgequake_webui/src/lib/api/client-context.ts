/**
 * @module api-client-context
 * @description Client-side session context for the API client: auth tokens,
 * tenant/workspace/user IDs, and W3C traceparent correlation.
 *
 * Extracted from `client.ts` (SPEC-017 LOC guard + SRP): all browser-stored
 * session state lives here. The request/response client and the streaming
 * client consume these helpers without owning the storage details.
 *
 * @implements FEAT0772 - Session context injection for API client
 * @implements SPEC-018 - W3C traceparent correlation
 */

const TRACEPARENT_STORAGE_KEY = "edgequake_traceparent";

// === Token management ==================================================

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

// === Tenant / workspace / user context ================================

let currentTenantId: string | null = null;
let currentWorkspaceId: string | null = null;
let currentUserId: string | null = null;

/** Generate a UUID v4 (crypto when available, Math.random fallback). */
function generateUUID(): string {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/** Get or create an anonymous user ID, persisted to localStorage. */
export function getOrCreateUserId(): string {
  if (typeof window !== "undefined") {
    let userId = localStorage.getItem("userId");
    if (!userId) {
      userId = generateUUID();
      localStorage.setItem("userId", userId);
    }
    return userId;
  }
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

// === W3C traceparent (SPEC-018) =======================================

/** Random lowercase hex string (cryptographic when available). */
function randomHex(byteLength: number): string {
  if (typeof crypto !== "undefined" && crypto.getRandomValues) {
    const bytes = new Uint8Array(byteLength);
    crypto.getRandomValues(bytes);
    return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
  }
  let out = "";
  for (let i = 0; i < byteLength * 2; i++) {
    out += Math.floor(Math.random() * 16).toString(16);
  }
  return out;
}

/**
 * W3C traceparent for browser → API correlation (SPEC-018).
 * Reuses the stored trace id from a prior API response when present.
 */
export function generateTraceparent(): string {
  if (typeof window !== "undefined") {
    const stored = sessionStorage.getItem(TRACEPARENT_STORAGE_KEY);
    if (stored?.startsWith("00-")) {
      const parts = stored.split("-");
      if (parts.length >= 4 && parts[1].length === 32) {
        return `00-${parts[1]}-${randomHex(8)}-01`;
      }
    }
  }
  return `00-${randomHex(16)}-${randomHex(8)}-01`;
}

/** Current browser trace context for client-side error logs (W3C). */
export function getClientTraceContext(): {
  traceparent: string | null;
  trace_id: string | null;
} {
  if (typeof window === "undefined") {
    return { traceparent: null, trace_id: null };
  }
  const traceparent = sessionStorage.getItem(TRACEPARENT_STORAGE_KEY);
  const trace_id =
    traceparent?.startsWith("00-") && traceparent.split("-")[1]?.length === 32
      ? traceparent.split("-")[1]
      : null;
  return { traceparent, trace_id };
}

/** Persist traceparent from an API response for subsequent requests. */
export function adoptTraceparentFromResponse(response: Response): void {
  const tp = response.headers?.get?.("traceparent");
  if (tp?.startsWith("00-") && typeof window !== "undefined") {
    sessionStorage.setItem(TRACEPARENT_STORAGE_KEY, tp);
  }
}

/** Generate a UUID v4 (re-exported for the headers builder). */
export function generateRequestId(): string {
  return generateUUID();
}
