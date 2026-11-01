/**
 * Session refresh over the HttpOnly refresh cookie (SPEC-154).
 *
 * Split out of `client.ts` (SPEC-017 god-module guard): this is the only code in the transport
 * layer that talks to `/auth/refresh`, and it owns the single-flight coalescing, so it belongs
 * beside nothing else. Re-exported from `client.ts` for existing callers.
 *
 * The access token lives in memory; the refresh token is an HttpOnly cookie the JavaScript
 * cannot read, so "do I still have a session?" is answered by redeeming the cookie, never by
 * inspecting storage.
 */

import { getRuntimeApiBaseUrl } from "@/lib/runtime-config";
import {
  applyRestoredAccessSession,
  type RestoredAccessSession,
} from "@/stores/use-auth-store";
import { clearTokens } from "./client-context";

/** One in-flight refresh, shared by every concurrent caller. */
let refreshInFlight: Promise<RestoredAccessSession | null> | null = null;

/** Dispatch a browser event so AuthGuard can react to permanent auth failures. */
export function dispatchAuthFailure(): void {
  if (typeof window !== "undefined") {
    window.dispatchEvent(new Event("auth:logout-required"));
  }
}

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
    // A server that omits `expires_in` still gets a usable session; 900s is the documented TTL.
    const expiresIn =
      typeof data.expires_in === "number" && data.expires_in > 0 ? data.expires_in : 900;
    const session: RestoredAccessSession = {
      access_token: data.access_token,
      expires_in: expiresIn,
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
 * Coalesces concurrent callers onto one network round-trip. Does not clear tokens or dispatch
 * logout — callers decide. AuthGuard uses this before redirecting to `/login`; the 401 retry path
 * clears on failure.
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
