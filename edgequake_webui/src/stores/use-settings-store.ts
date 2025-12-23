"use client";

import type {
  AppSettings,
  GraphSettings,
  QueryMode,
  QuerySettings,
} from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

const defaultGraphSettings: GraphSettings = {
  showLabels: true,
  showEdgeLabels: false,
  nodeSize: "medium",
  edgeThickness: "medium",
  layout: "force",
  colorBy: "type",
  enableNodeDrag: true,
  highlightNeighbors: true,
  hideUnselectedEdges: false,
};

const defaultQuerySettings: QuerySettings = {
  mode: "hybrid" as QueryMode,
  topK: 10,
  maxTokens: 2048,
  temperature: 0.7,
  stream: true, // Enable streaming by default for better UX
};

interface SettingsState extends AppSettings {
  // Sidebar state
  sidebarCollapsed: boolean;
  // Actions
  setTheme: (theme: AppSettings["theme"]) => void;
  setLanguage: (language: AppSettings["language"]) => void;
  setGraphSettings: (settings: Partial<GraphSettings>) => void;
  setQuerySettings: (settings: Partial<QuerySettings>) => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  toggleSidebar: () => void;
  resetSettings: () => void;
}

const initialState: AppSettings & { sidebarCollapsed: boolean } = {
  theme: "system",
  language: "en",
  graphSettings: defaultGraphSettings,
  querySettings: defaultQuerySettings,
  sidebarCollapsed: false,
};

export const useSettingsStore = create<SettingsState>()(
  persist(
    (set) => ({
      ...initialState,

      setTheme: (theme) => set({ theme }),

      setLanguage: (language) => set({ language }),

      setGraphSettings: (settings) =>
        set((state) => ({
          graphSettings: { ...state.graphSettings, ...settings },
        })),

      setQuerySettings: (settings) =>
        set((state) => ({
          querySettings: { ...state.querySettings, ...settings },
        })),

      setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),

      toggleSidebar: () =>
        set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),

      resetSettings: () => set(initialState),
    }),
    {
      name: "edgequake-settings",
      partialize: (state) => ({
        theme: state.theme,
        language: state.language,
        graphSettings: state.graphSettings,
        querySettings: state.querySettings,
        sidebarCollapsed: state.sidebarCollapsed,
      }),
    }
  )
);

export default useSettingsStore;
