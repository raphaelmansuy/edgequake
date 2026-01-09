/**
 * @module storage-keys
 * @description Centralized Storage Keys
 *
 * Single source of truth for all localStorage keys used in EdgeQuake WebUI.
 * This prevents typos, enables easy refactoring, and documents all persistent state.
 *
 * @implements FEAT0731 - Centralized storage key management
 * @implements FEAT0732 - Storage migration support
 *
 * @enforces BR0728 - All keys prefixed with 'edgequake-'
 * @enforces BR0729 - Never store sensitive data in localStorage
 */

/**
 * Zustand store persistence keys
 */
export const ZUSTAND_STORAGE_KEYS = {
  /** Tenant and workspace selection state */
  TENANT_STORE: "edgequake-tenant",

  /** Authentication state (user info, expiry - NOT tokens) */
  AUTH_STORE: "edgequake-auth",

  /** Application settings (theme, language, graph/query settings) */
  SETTINGS_STORE: "edgequake-settings",

  /** Query history and favorites */
  QUERY_STORE: "edgequake-query",

  /** Query UI state (panel visibility, filters, streaming) */
  QUERY_UI_STORE: "edgequake-query-ui",

  /** Conversation history with messages */
  CONVERSATION_STORE: "edgequake-conversations",

  /** Cost tracking and budget status */
  COST_STORE: "edgequake-cost",

  /** UI preferences (panel states, view modes, widths) */
  UI_PREFERENCES: "edgequake-ui-preferences",
} as const;

/**
 * Legacy storage keys (to be deprecated)
 *
 * These keys were used for direct localStorage access before
 * migrating to Zustand. They are kept for backward compatibility
 * during the migration period.
 *
 * @deprecated Use Zustand stores instead
 */
export const LEGACY_STORAGE_KEYS = {
  /** @deprecated Use ZUSTAND_STORAGE_KEYS.TENANT_STORE */
  TENANT_ID: "tenantId",

  /** @deprecated Use ZUSTAND_STORAGE_KEYS.TENANT_STORE */
  WORKSPACE_ID: "workspaceId",

  /** @deprecated Use ZUSTAND_STORAGE_KEYS.AUTH_STORE */
  ACCESS_TOKEN: "accessToken",

  /** @deprecated Use ZUSTAND_STORAGE_KEYS.AUTH_STORE */
  REFRESH_TOKEN: "refreshToken",

  /** Anonymous user identifier - still used directly */
  USER_ID: "userId",

  /** Old conversations format before server-side storage */
  OLD_CONVERSATIONS: "edgequake-conversations",
} as const;

/**
 * Application flag keys
 *
 * These are one-time flags or non-store persistent values.
 */
export const FLAG_STORAGE_KEYS = {
  /** Tracks if workspace auto-selection toast has been shown */
  WORKSPACE_INITIALIZED: "edgequake-workspace-initialized",

  /** Tracks if conversation migration from localStorage to server has completed */
  CONVERSATIONS_MIGRATED: "edgequake-conversations-migrated",

  /** Storage version for detecting schema changes */
  STORAGE_VERSION: "edgequake-storage-version",
} as const;

/**
 * Third-party integration keys
 */
export const INTEGRATION_STORAGE_KEYS = {
  /** i18next language preference */
  LANGUAGE: "edgequake-language",
} as const;

/**
 * Cache keys (can be safely cleared)
 */
export const CACHE_STORAGE_KEYS = {
  /** Graph visualization cache */
  GRAPH_CACHE: "edgequake-graph-cache",

  /** Query history cache */
  QUERY_HISTORY: "edgequake-query-history",
} as const;

/**
 * All storage keys combined
 */
export const STORAGE_KEYS = {
  ...ZUSTAND_STORAGE_KEYS,
  ...LEGACY_STORAGE_KEYS,
  ...FLAG_STORAGE_KEYS,
  ...INTEGRATION_STORAGE_KEYS,
  ...CACHE_STORAGE_KEYS,
} as const;

/**
 * Current storage schema version
 *
 * Increment this when making breaking changes to stored data structure.
 * Migration logic should handle upgrading from previous versions.
 */
export const CURRENT_STORAGE_VERSION = 1;

/**
 * Store versions for Zustand persist middleware
 *
 * Each store tracks its own version independently.
 */
export const STORE_VERSIONS = {
  [ZUSTAND_STORAGE_KEYS.TENANT_STORE]: 1,
  [ZUSTAND_STORAGE_KEYS.AUTH_STORE]: 1,
  [ZUSTAND_STORAGE_KEYS.SETTINGS_STORE]: 1,
  [ZUSTAND_STORAGE_KEYS.QUERY_STORE]: 1,
  [ZUSTAND_STORAGE_KEYS.QUERY_UI_STORE]: 1,
  [ZUSTAND_STORAGE_KEYS.CONVERSATION_STORE]: 1,
  [ZUSTAND_STORAGE_KEYS.COST_STORE]: 1,
} as const;

/**
 * Get all keys that should be cleared on logout
 */
export function getLogoutClearKeys(): string[] {
  return [
    ZUSTAND_STORAGE_KEYS.TENANT_STORE,
    ZUSTAND_STORAGE_KEYS.AUTH_STORE,
    ZUSTAND_STORAGE_KEYS.QUERY_STORE,
    ZUSTAND_STORAGE_KEYS.QUERY_UI_STORE,
    ZUSTAND_STORAGE_KEYS.CONVERSATION_STORE,
    ZUSTAND_STORAGE_KEYS.COST_STORE,
    LEGACY_STORAGE_KEYS.ACCESS_TOKEN,
    LEGACY_STORAGE_KEYS.REFRESH_TOKEN,
    LEGACY_STORAGE_KEYS.TENANT_ID,
    LEGACY_STORAGE_KEYS.WORKSPACE_ID,
    FLAG_STORAGE_KEYS.WORKSPACE_INITIALIZED,
  ];
}

/**
 * Get all keys that can be safely cleared as cache
 */
export function getCacheClearKeys(): string[] {
  return [CACHE_STORAGE_KEYS.GRAPH_CACHE, CACHE_STORAGE_KEYS.QUERY_HISTORY];
}

/**
 * Clear all application storage (for reset functionality)
 */
export function clearAllStorage(): void {
  if (typeof window === "undefined") return;

  const allKeys = [
    ...Object.values(ZUSTAND_STORAGE_KEYS),
    ...Object.values(LEGACY_STORAGE_KEYS),
    ...Object.values(FLAG_STORAGE_KEYS),
    ...Object.values(CACHE_STORAGE_KEYS),
  ];

  allKeys.forEach((key) => {
    try {
      localStorage.removeItem(key);
    } catch {
      // Ignore errors (e.g., in SSR or when localStorage is disabled)
    }
  });
}

export default STORAGE_KEYS;
