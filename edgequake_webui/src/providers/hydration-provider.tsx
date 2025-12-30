/**
 * Hydration Provider
 *
 * Gates the application until critical Zustand stores have hydrated
 * from localStorage. This prevents hydration mismatches and ensures
 * the app doesn't render with stale/default state.
 *
 * NOTE: This provider is available for opt-in usage but is NOT in the
 * main provider hierarchy. Individual stores handle their own hydration
 * via `_hasHydrated` state and TenantGuard handles loading states.
 *
 * The setState-in-effect pattern is intentional here for detecting
 * Zustand persist hydration completion which uses async callbacks.
 *
 * @module hydration-provider
 */

/* eslint-disable react-hooks/set-state-in-effect */

'use client';

import { useAuthStore } from '@/stores/use-auth-store';
import { useSettingsStore } from '@/stores/use-settings-store';
import { useTenantStore } from '@/stores/use-tenant-store';
import { type ReactNode, useEffect, useState } from 'react';

interface HydrationProviderProps {
  children: ReactNode;
  /**
   * Optional loading component to show during hydration
   */
  fallback?: ReactNode;
}

/**
 * Critical stores that must hydrate before rendering
 *
 * These stores contain state that affects the initial render:
 * - Tenant/Workspace: Determines API context
 * - Auth: Determines if user is logged in
 * - Settings: Determines theme (prevents flash)
 */
const CRITICAL_STORES = [
  useTenantStore,
  useAuthStore,
  useSettingsStore,
] as const;

/**
 * Default loading skeleton
 */
function DefaultLoadingFallback() {
  return (
    <div className="flex h-screen w-screen items-center justify-center bg-background">
      <div className="flex flex-col items-center gap-4">
        <div className="h-8 w-8 animate-spin rounded-full border-4 border-primary border-t-transparent" />
        <p className="text-sm text-muted-foreground">Loading...</p>
      </div>
    </div>
  );
}

/**
 * Check if a store has persist middleware and is hydrated
 */
function isStoreHydrated(store: { persist?: { hasHydrated: () => boolean } }): boolean {
  // If no persist middleware, it's always "hydrated"
  if (!store.persist) return true;
  return store.persist.hasHydrated();
}

/**
 * HydrationProvider - Gates app rendering until stores hydrate
 *
 * This provider solves the SSR hydration mismatch problem by:
 * 1. Rendering a loading state on first mount (matches server)
 * 2. Waiting for critical stores to hydrate from localStorage
 * 3. Only then rendering children with correct state
 *
 * @example
 * ```tsx
 * // In providers/index.tsx
 * <HydrationProvider fallback={<CustomSkeleton />}>
 *   <TenantProvider>
 *     {children}
 *   </TenantProvider>
 * </HydrationProvider>
 * ```
 */
export function HydrationProvider({
  children,
  fallback = <DefaultLoadingFallback />,
}: HydrationProviderProps) {
  // Start with false to match server render
  const [isHydrated, setIsHydrated] = useState(false);

  useEffect(() => {
    // Check if all critical stores are hydrated
    const checkHydration = () => {
      return CRITICAL_STORES.every((store) => isStoreHydrated(store));
    };

    // If already hydrated, set state immediately
    if (checkHydration()) {
      setIsHydrated(true);
      return;
    }

    // Set up listeners for each store's hydration completion
    const unsubscribers: (() => void)[] = [];

    CRITICAL_STORES.forEach((store) => {
      if (store.persist) {
        const unsub = store.persist.onFinishHydration(() => {
          if (checkHydration()) {
            setIsHydrated(true);
          }
        });
        unsubscribers.push(unsub);
      }
    });

    // Also check with a small delay as a safety net
    const timeoutId = setTimeout(() => {
      if (checkHydration()) {
        setIsHydrated(true);
      }
    }, 100);

    return () => {
      unsubscribers.forEach((unsub) => unsub());
      clearTimeout(timeoutId);
    };
  }, []);

  // On first render (SSR or initial client), show fallback
  // This ensures server and client render the same thing initially
  if (!isHydrated) {
    return <>{fallback}</>;
  }

  return <>{children}</>;
}

/**
 * Hook to check if app has completed hydration
 *
 * Useful for components that need to know hydration status
 * without being gated by HydrationProvider.
 *
 * @example
 * ```tsx
 * const isAppHydrated = useAppHydrated();
 *
 * if (!isAppHydrated) {
 *   return <Skeleton />;
 * }
 *
 * return <ActualComponent />;
 * ```
 */
export function useAppHydrated(): boolean {
  const [isHydrated, setIsHydrated] = useState(false);

  useEffect(() => {
    const checkHydration = () => {
      return CRITICAL_STORES.every((store) => isStoreHydrated(store));
    };

    if (checkHydration()) {
      setIsHydrated(true);
      return;
    }

    const unsubscribers = CRITICAL_STORES
      .filter((store) => store.persist)
      .map((store) =>
        store.persist!.onFinishHydration(() => {
          if (checkHydration()) {
            setIsHydrated(true);
          }
        })
      );

    // Safety net timeout
    const timeoutId = setTimeout(() => {
      if (checkHydration()) {
        setIsHydrated(true);
      }
    }, 100);

    return () => {
      unsubscribers.forEach((unsub) => unsub());
      clearTimeout(timeoutId);
    };
  }, []);

  return isHydrated;
}

export default HydrationProvider;
