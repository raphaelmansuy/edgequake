/**
 * @module use-ui-preferences-store
 * @description Zustand store for UI layout preferences.
 * Persists panel states, widths, and view modes to localStorage.
 *
 * @implements FEAT0622 - Persistent panel collapse states
 * @implements FEAT0623 - Resizable panel widths
 * @implements FEAT0624 - View mode preferences (grouped/list)
 *
 * @enforces BR0610 - Panel states persist across sessions
 * @enforces BR0614 - Width preferences within min/max bounds
 *
 * @see {@link docs/features.md} FEAT0622-0624
 */
"use client";

import { ZUSTAND_STORAGE_KEYS } from "@/lib/storage-keys";
import { create } from "zustand";
import { createJSONStorage, persist } from "zustand/middleware";

/**
 * UI Preferences Store
 *
 * Persists user interface preferences to localStorage, including:
 * - Panel collapsed states
 * - Layout preferences
 * - View mode selections
 */

interface UIPanelState {
  // Graph page panels
  graphEntityBrowserCollapsed: boolean;
  graphDetailsPanelCollapsed: boolean;
  graphEntityBrowserWidth: number;
  graphDetailsPanelWidth: number;

  // Documents page
  documentsPreviewCollapsed: boolean;
  documentsPreviewWidth: number;

  // Query page
  queryHistoryCollapsed: boolean;
  queryHistoryWidth: number;

  // Entity browser view mode
  entityBrowserViewMode: "grouped" | "list";
  entityBrowserSortBy: "name" | "degree" | "type";
  entityBrowserSortAsc: boolean;
}

interface UIPreferencesState extends UIPanelState {
  _hasHydrated: boolean;

  // Actions
  setGraphEntityBrowserCollapsed: (collapsed: boolean) => void;
  setGraphDetailsPanelCollapsed: (collapsed: boolean) => void;
  setGraphEntityBrowserWidth: (width: number) => void;
  setGraphDetailsPanelWidth: (width: number) => void;

  setDocumentsPreviewCollapsed: (collapsed: boolean) => void;
  setDocumentsPreviewWidth: (width: number) => void;

  setQueryHistoryCollapsed: (collapsed: boolean) => void;
  setQueryHistoryWidth: (width: number) => void;

  setEntityBrowserViewMode: (mode: "grouped" | "list") => void;
  setEntityBrowserSortBy: (sortBy: "name" | "degree" | "type") => void;
  setEntityBrowserSortAsc: (asc: boolean) => void;

  // Hydration
  setHasHydrated: (hydrated: boolean) => void;

  // Reset
  resetUIPreferences: () => void;
}

const initialState: UIPanelState & { _hasHydrated: boolean } = {
  // Graph page panels
  graphEntityBrowserCollapsed: false,
  graphDetailsPanelCollapsed: false,
  graphEntityBrowserWidth: 256, // w-64
  graphDetailsPanelWidth: 320,

  // Documents page
  documentsPreviewCollapsed: true,
  documentsPreviewWidth: 400,

  // Query page
  queryHistoryCollapsed: false,
  queryHistoryWidth: 280,

  // Entity browser defaults
  entityBrowserViewMode: "grouped",
  entityBrowserSortBy: "name",
  entityBrowserSortAsc: true,

  _hasHydrated: false,
};

export const useUIPreferencesStore = create<UIPreferencesState>()(
  persist(
    (set) => ({
      ...initialState,

      // Graph page
      setGraphEntityBrowserCollapsed: (collapsed) =>
        set({ graphEntityBrowserCollapsed: collapsed }),
      setGraphDetailsPanelCollapsed: (collapsed) =>
        set({ graphDetailsPanelCollapsed: collapsed }),
      setGraphEntityBrowserWidth: (width) =>
        set({ graphEntityBrowserWidth: width }),
      setGraphDetailsPanelWidth: (width) =>
        set({ graphDetailsPanelWidth: width }),

      // Documents page
      setDocumentsPreviewCollapsed: (collapsed) =>
        set({ documentsPreviewCollapsed: collapsed }),
      setDocumentsPreviewWidth: (width) =>
        set({ documentsPreviewWidth: width }),

      // Query page
      setQueryHistoryCollapsed: (collapsed) =>
        set({ queryHistoryCollapsed: collapsed }),
      setQueryHistoryWidth: (width) => set({ queryHistoryWidth: width }),

      // Entity browser
      setEntityBrowserViewMode: (mode) => set({ entityBrowserViewMode: mode }),
      setEntityBrowserSortBy: (sortBy) => set({ entityBrowserSortBy: sortBy }),
      setEntityBrowserSortAsc: (asc) => set({ entityBrowserSortAsc: asc }),

      // Hydration
      setHasHydrated: (hydrated) => set({ _hasHydrated: hydrated }),

      // Reset
      resetUIPreferences: () => set(initialState),
    }),
    {
      name: ZUSTAND_STORAGE_KEYS.UI_PREFERENCES,
      storage: createJSONStorage(() => localStorage),
      version: 1,
      onRehydrateStorage: () => (state) => {
        state?.setHasHydrated(true);
      },
      partialize: (state) => ({
        // Only persist UI preferences, not hydration state
        graphEntityBrowserCollapsed: state.graphEntityBrowserCollapsed,
        graphDetailsPanelCollapsed: state.graphDetailsPanelCollapsed,
        graphEntityBrowserWidth: state.graphEntityBrowserWidth,
        graphDetailsPanelWidth: state.graphDetailsPanelWidth,
        documentsPreviewCollapsed: state.documentsPreviewCollapsed,
        documentsPreviewWidth: state.documentsPreviewWidth,
        queryHistoryCollapsed: state.queryHistoryCollapsed,
        queryHistoryWidth: state.queryHistoryWidth,
        entityBrowserViewMode: state.entityBrowserViewMode,
        entityBrowserSortBy: state.entityBrowserSortBy,
        entityBrowserSortAsc: state.entityBrowserSortAsc,
      }),
    }
  )
);

export default useUIPreferencesStore;
