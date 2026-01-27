/**
 * @fileoverview Cache manager for React Query and localStorage
 *
 * @implements FEAT0865 - Aggressive cache invalidation
 * @implements FEAT0866 - Cache versioning for stale detection
 *
 * WHY: Prevent stale cache from showing incorrect stats
 *
 * PROBLEM:
 * - React Query cache persists across page refreshes
 * - When workspace changes, old cached stats remain
 * - Dashboard shows 0 entities from old workspace instead of fresh data
 *
 * SOLUTION:
 * - Version-based cache invalidation
 * - Automatic cache clearing on workspace/tenant change
 * - Force fresh fetch when version mismatch detected
 */

"use client";

import type { QueryClient } from "@tanstack/react-query";

const CACHE_VERSION_KEY = "edgequake-cache-version";
const CACHE_VERSION = "v1.0.0"; // Increment this to force cache clear

interface CacheContext {
  tenantId: string | null;
  workspaceId: string | null;
  version: string;
  timestamp: number;
}

/**
 * Get current cache context from localStorage
 */
export function getCacheContext(): CacheContext | null {
  if (typeof window === "undefined") return null;

  const stored = localStorage.getItem(CACHE_VERSION_KEY);
  if (!stored) return null;

  try {
    return JSON.parse(stored) as CacheContext;
  } catch {
    return null;
  }
}

/**
 * Save current cache context to localStorage
 */
export function saveCacheContext(context: CacheContext): void {
  if (typeof window === "undefined") return;
  localStorage.setItem(CACHE_VERSION_KEY, JSON.stringify(context));
}

/**
 * Check if cache is stale and needs invalidation
 *
 * Cache is stale if:
 * 1. Version mismatch (code update)
 * 2. Tenant changed
 * 3. Workspace changed
 * 4. Cache older than 1 hour
 */
export function isCacheStale(
  currentTenantId: string | null,
  currentWorkspaceId: string | null,
): boolean {
  const context = getCacheContext();

  // No context = first load, cache is "stale" (needs init)
  if (!context) return true;

  // Version mismatch = code update, clear cache
  if (context.version !== CACHE_VERSION) {
    return true;
  }

  // Tenant changed = clear cache
  if (context.tenantId !== currentTenantId) {
    return true;
  }

  // Workspace changed = clear cache
  if (context.workspaceId !== currentWorkspaceId) {
    return true;
  }

  // Cache older than 1 hour = stale
  const ONE_HOUR = 60 * 60 * 1000;
  if (Date.now() - context.timestamp > ONE_HOUR) {
    return true;
  }

  return false;
}

/**
 * Clear all React Query caches
 */
export function clearQueryCache(queryClient: QueryClient): void {
  // Clear all queries
  queryClient.clear();

  // Force invalidation of specific keys
  queryClient.invalidateQueries({ queryKey: ["workspaceStats"] });
  queryClient.invalidateQueries({ queryKey: ["documents"] });
  queryClient.invalidateQueries({ queryKey: ["graph"] });
  queryClient.invalidateQueries({ queryKey: ["workspaces"] });
  queryClient.invalidateQueries({ queryKey: ["tenants"] });
}

/**
 * Clear localStorage cache entries (except auth tokens and user ID)
 */
export function clearLocalStorageCache(): void {
  if (typeof window === "undefined") return;

  // Keep these keys
  const keepKeys = [
    "accessToken",
    "refreshToken",
    "userId",
    "theme",
    "language",
  ];

  // Remove all other keys
  const keysToRemove: string[] = [];
  for (let i = 0; i < localStorage.length; i++) {
    const key = localStorage.key(i);
    if (key && !keepKeys.includes(key)) {
      keysToRemove.push(key);
    }
  }

  keysToRemove.forEach((key) => {
    localStorage.removeItem(key);
  });
}

/**
 * Validate cache and clear if stale
 *
 * Call this on app initialization or when workspace/tenant changes
 */
export function validateAndClearCache(
  queryClient: QueryClient,
  tenantId: string | null,
  workspaceId: string | null,
): void {
  const isStale = isCacheStale(tenantId, workspaceId);

  if (isStale) {
    console.warn("[CacheManager] Cache is stale, clearing all caches", {
      tenantId,
      workspaceId,
    });

    // Clear React Query cache
    clearQueryCache(queryClient);

    // Clear localStorage cache (except auth)
    clearLocalStorageCache();
  }

  // Update cache context
  saveCacheContext({
    tenantId,
    workspaceId,
    version: CACHE_VERSION,
    timestamp: Date.now(),
  });
}

/**
 * Force cache clear (for debugging or manual reset)
 */
export function forceCacheClear(queryClient: QueryClient): void {
  console.warn("[CacheManager] FORCE CLEAR requested");

  clearQueryCache(queryClient);
  clearLocalStorageCache();

  // Reset version context
  if (typeof window !== "undefined") {
    localStorage.removeItem(CACHE_VERSION_KEY);
  }
}
