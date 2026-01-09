/**
 * @module use-store-hydration
 * @description Zustand Hydration Utilities
 *
 * Provides SSR-safe hooks for accessing Zustand persisted stores.
 * Prevents hydration mismatches in Next.js by properly handling
 * the async nature of localStorage hydration.
 *
 * @implements FEAT0649 - SSR-safe store hydration
 * @implements FEAT0650 - Hydration mismatch prevention
 *
 * @enforces BR0634 - Server renders default state
 * @enforces BR0635 - Client hydrates from localStorage
 */

"use client";

import { useEffect, useState, useSyncExternalStore } from "react";
import type { StoreApi, UseBoundStore } from "zustand";

/**
 * Generic type for stores with persist middleware
 */
interface PersistStore {
  persist: {
    hasHydrated: () => boolean;
    onHydrate: (listener: () => void) => () => void;
    onFinishHydration: (listener: () => void) => () => void;
    rehydrate: () => Promise<void>;
  };
}

/**
 * Hook to track hydration state of a Zustand persist store
 *
 * @example
 * ```tsx
 * const hydrated = useStoreHydrated(useTenantStore);
 *
 * if (!hydrated) {
 *   return <LoadingSkeleton />;
 * }
 *
 * return <TenantSelector />;
 * ```
 */
export function useStoreHydrated<T>(
  store: UseBoundStore<StoreApi<T>> & Partial<PersistStore>
): boolean {
  // Check if store has persist middleware
  const hasPersist = Boolean(store.persist);

  // Use useSyncExternalStore for proper SSR handling
  // Note: We always call the hook, but provide no-op functions for non-persisted stores
  const hydrated = useSyncExternalStore(
    (onStoreChange) => {
      // For non-persisted stores, return a no-op unsubscribe
      if (!hasPersist || !store.persist) {
        return () => {};
      }
      // Subscribe to hydration events
      const unsub = store.persist.onFinishHydration(onStoreChange);
      return unsub;
    },
    () => {
      // For non-persisted stores, always return true
      if (!hasPersist || !store.persist) {
        return true;
      }
      return store.persist.hasHydrated();
    },
    () => {
      // For non-persisted stores, always return true on server
      // For persisted stores, server is never hydrated
      return !hasPersist;
    }
  );

  return hydrated;
}

/**
 * Hook to safely access persisted store data in SSR
 *
 * Returns undefined until hydration is complete, then returns the selected value.
 * This prevents hydration mismatches by ensuring server and client render the same initially.
 *
 * @example
 * ```tsx
 * const selectedTenantId = useHydratedStore(
 *   useTenantStore,
 *   (state) => state.selectedTenantId
 * );
 *
 * // selectedTenantId is undefined on first render, then the actual value
 * ```
 */
export function useHydratedStore<T, S>(
  store: UseBoundStore<StoreApi<T>> & Partial<PersistStore>,
  selector: (state: T) => S
): S | undefined {
  const result = store(selector);
  const [data, setData] = useState<S | undefined>(undefined);

  useEffect(() => {
    setData(result);
  }, [result]);

  return data;
}

/**
 * Hook using useSyncExternalStore for optimal SSR handling
 *
 * This is the recommended approach for React 18+ as it properly
 * handles concurrent rendering and hydration.
 *
 * @example
 * ```tsx
 * const tenantId = useSyncStore(
 *   useTenantStore,
 *   (state) => state.selectedTenantId,
 *   null // Server-side fallback
 * );
 * ```
 */
export function useSyncStore<T, S>(
  store: UseBoundStore<StoreApi<T>>,
  selector: (state: T) => S,
  serverFallback: S
): S {
  const getSnapshot = () => selector(store.getState());
  const getServerSnapshot = () => serverFallback;

  return useSyncExternalStore(store.subscribe, getSnapshot, getServerSnapshot);
}

/**
 * Wait for all specified stores to hydrate
 *
 * @example
 * ```tsx
 * const allHydrated = useAllStoresHydrated([
 *   useTenantStore,
 *   useAuthStore,
 *   useSettingsStore,
 * ]);
 *
 * if (!allHydrated) {
 *   return <AppLoadingScreen />;
 * }
 * ```
 */
export function useAllStoresHydrated(
  stores: (UseBoundStore<StoreApi<unknown>> & Partial<PersistStore>)[]
): boolean {
  // Create a combined subscription for all stores
  const subscribe = (onStoreChange: () => void) => {
    const unsubscribers = stores
      .filter((store) => store.persist)
      .map((store) => store.persist!.onFinishHydration(onStoreChange));

    return () => {
      unsubscribers.forEach((unsub) => unsub());
    };
  };

  const getSnapshot = () => {
    return stores.every((store) => {
      if (!store.persist) return true;
      return store.persist.hasHydrated();
    });
  };

  const getServerSnapshot = () => false;

  return useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
}

/**
 * Force rehydrate a store from localStorage
 *
 * Useful when localStorage is modified externally (e.g., in another tab)
 *
 * @example
 * ```tsx
 * const rehydrate = useRehydrateStore(useTenantStore);
 *
 * // After external localStorage change:
 * await rehydrate();
 * ```
 */
export function useRehydrateStore<T>(
  store: UseBoundStore<StoreApi<T>> & Partial<PersistStore>
): () => Promise<void> {
  return async () => {
    if (store.persist) {
      await store.persist.rehydrate();
    }
  };
}

/**
 * Listen to localStorage changes from other tabs/windows
 *
 * Automatically rehydrates the store when its key changes externally.
 *
 * @example
 * ```tsx
 * useCrossTabSync(useTenantStore, 'edgequake-tenant');
 * ```
 */
export function useCrossTabSync<T>(
  store: UseBoundStore<StoreApi<T>> & Partial<PersistStore>,
  storageKey: string
): void {
  useEffect(() => {
    if (typeof window === "undefined" || !store.persist) {
      return;
    }

    const handleStorageChange = (event: StorageEvent) => {
      if (event.key === storageKey && event.newValue !== null) {
        store.persist!.rehydrate();
      }
    };

    window.addEventListener("storage", handleStorageChange);

    return () => {
      window.removeEventListener("storage", handleStorageChange);
    };
  }, [store, storageKey]);
}

export default useStoreHydrated;
