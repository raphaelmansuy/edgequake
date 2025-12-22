"use client";

import { getTenantContext, setTenantContext } from "@/lib/api/client";
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
}

type TenantStore = TenantState & TenantActions;

const initialState: TenantState = {
  tenants: [],
  workspaces: [],
  selectedTenantId: null,
  selectedWorkspaceId: null,
  isLoading: false,
  error: null,
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
      name: "edgequake-tenant",
      partialize: (state) => ({
        selectedTenantId: state.selectedTenantId,
        selectedWorkspaceId: state.selectedWorkspaceId,
      }),
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

export default useTenantStore;
