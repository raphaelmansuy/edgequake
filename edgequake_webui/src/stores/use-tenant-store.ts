/**
 * @module use-tenant-store
 * @description Zustand store for multi-tenant context management.
 * Manages tenant/workspace selection and provides API context headers.
 *
 * @implements UC0506 - User selects tenant from available list
 * @implements UC0507 - User selects workspace within tenant
 * @implements FEAT0504 - Multi-tenancy with workspace isolation
 * @implements FEAT0506 - Tenant context persisted across sessions
 *
 * @enforces BR0504 - All API calls include tenant/workspace headers
 * @enforces BR0506 - Switching workspace clears stale data
 * @enforces BR0507 - New users trigger onboarding flow
 *
 * @see {@link docs/features.md} FEAT0504, FEAT0506
 */
"use client";

import { getTenantContext, setTenantContext } from "@/lib/api/client";
import {
  LEGACY_STORAGE_KEYS,
  STORE_VERSIONS,
  ZUSTAND_STORAGE_KEYS,
} from "@/lib/storage-keys";
import type { Tenant, Workspace } from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface TenantState {
  tenants: Tenant[];
  workspaces: Workspace[];
  selectedTenantId: string | null;
  selectedWorkspaceId: string | null;
  isLoading: boolean;
  error: string | null;
  isInitialized: boolean;
  needsOnboarding: boolean;
  /** Tracks if store has been hydrated from localStorage */
  _hasHydrated: boolean;
}

interface TenantActions {
  setTenants: (tenants: Tenant[]) => void;
  setWorkspaces: (workspaces: Workspace[]) => void;
  selectTenant: (tenantId: string) => void;
  selectWorkspace: (workspaceId: string) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
  initializeFromStorage: () => void;
  setInitialized: (initialized: boolean) => void;
  setNeedsOnboarding: (needs: boolean) => void;
  setHasHydrated: (hydrated: boolean) => void;
}

type TenantStore = TenantState & TenantActions;

const initialState: TenantState = {
  tenants: [],
  workspaces: [],
  selectedTenantId: null,
  selectedWorkspaceId: null,
  isLoading: false,
  error: null,
  isInitialized: false,
  needsOnboarding: false,
  _hasHydrated: false,
};

export const useTenantStore = create<TenantStore>()(
  persist(
    (set, get) => ({
      ...initialState,

      setTenants: (tenants) => set({ tenants }),

      setWorkspaces: (workspaces) => set({ workspaces }),

      selectTenant: (tenantId) => {
        set({
          selectedTenantId: tenantId,
          selectedWorkspaceId: null,
          workspaces: [],
        });
        setTenantContext(tenantId);
      },

      selectWorkspace: (workspaceId) => {
        const { selectedTenantId } = get();
        set({ selectedWorkspaceId: workspaceId });
        if (selectedTenantId) {
          setTenantContext(selectedTenantId, workspaceId);
        }
      },

      setLoading: (loading) => set({ isLoading: loading }),

      setError: (error) => set({ error }),

      setInitialized: (initialized) => set({ isInitialized: initialized }),

      setNeedsOnboarding: (needs) => set({ needsOnboarding: needs }),

      setHasHydrated: (hydrated) => set({ _hasHydrated: hydrated }),

      reset: () => set(initialState),

      initializeFromStorage: () => {
        const { tenantId, workspaceId } = getTenantContext();
        if (tenantId) {
          set({
            selectedTenantId: tenantId,
            selectedWorkspaceId: workspaceId,
          });
        }
      },
    }),
    {
      name: ZUSTAND_STORAGE_KEYS.TENANT_STORE,
      version: STORE_VERSIONS[ZUSTAND_STORAGE_KEYS.TENANT_STORE],
      partialize: (state) => ({
        selectedTenantId: state.selectedTenantId,
        selectedWorkspaceId: state.selectedWorkspaceId,
      }),
      /**
       * Migration function for handling schema changes
       *
       * Version 0 -> 1: Migrate from legacy localStorage keys
       */
      migrate: (persistedState: unknown, version: number) => {
        const state = persistedState as Partial<TenantState>;

        if (version === 0) {
          // Migrate from legacy keys if they exist
          if (typeof window !== "undefined") {
            const legacyTenantId = localStorage.getItem(
              LEGACY_STORAGE_KEYS.TENANT_ID
            );
            const legacyWorkspaceId = localStorage.getItem(
              LEGACY_STORAGE_KEYS.WORKSPACE_ID
            );

            if (legacyTenantId && !state.selectedTenantId) {
              state.selectedTenantId = legacyTenantId;
            }
            if (legacyWorkspaceId && !state.selectedWorkspaceId) {
              state.selectedWorkspaceId = legacyWorkspaceId;
            }

            // Clean up legacy keys after migration
            // Note: We keep them for now to maintain backward compat with client.ts
            // TODO: Remove once client.ts dual storage is fixed
          }
        }

        return state as TenantState;
      },
      /**
       * Callback when hydration starts/finishes
       * Used to track hydration state for SSR safety
       */
      onRehydrateStorage: () => {
        return (state, error) => {
          if (error) {
            console.error("[TenantStore] Hydration failed:", error);
          }
          // Mark as hydrated even on error (to prevent infinite loading)
          state?.setHasHydrated(true);

          // Sync to API client after hydration
          if (state?.selectedTenantId) {
            setTenantContext(
              state.selectedTenantId,
              state.selectedWorkspaceId ?? undefined
            );
          }
        };
      },
    }
  )
);

// Selectors
export const useSelectedTenant = () => {
  const { tenants, selectedTenantId } = useTenantStore();
  return tenants.find((t) => t.id === selectedTenantId) || null;
};

export const useSelectedWorkspace = () => {
  const { workspaces, selectedWorkspaceId } = useTenantStore();
  return workspaces.find((w) => w.id === selectedWorkspaceId) || null;
};

/**
 * Selector for hydration state
 * Returns true once the store has hydrated from localStorage
 */
export const useTenantStoreHydrated = () => {
  return useTenantStore((state) => state._hasHydrated);
};

/**
 * Check if a valid context is selected
 * Useful for gating API calls that require tenant/workspace context
 */
export const useHasValidContext = () => {
  const { selectedTenantId, selectedWorkspaceId, _hasHydrated } =
    useTenantStore();
  return _hasHydrated && !!selectedTenantId && !!selectedWorkspaceId;
};

export default useTenantStore;
