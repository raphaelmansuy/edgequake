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
};

const defaultQuerySettings: QuerySettings = {
  mode: "hybrid" as QueryMode,
  topK: 10,
  maxTokens: 2048,
  temperature: 0.7,
  stream: false,
};

interface SettingsState extends AppSettings {
  // Actions
  setTheme: (theme: AppSettings["theme"]) => void;
  setLanguage: (language: AppSettings["language"]) => void;
  setGraphSettings: (settings: Partial<GraphSettings>) => void;
  setQuerySettings: (settings: Partial<QuerySettings>) => void;
  resetSettings: () => void;
}

const initialState: AppSettings = {
  theme: "system",
  language: "en",
  graphSettings: defaultGraphSettings,
  querySettings: defaultQuerySettings,
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

      resetSettings: () => set(initialState),
    }),
    {
      name: "edgequake-settings",
      partialize: (state) => ({
        theme: state.theme,
        language: state.language,
        graphSettings: state.graphSettings,
        querySettings: state.querySettings,
      }),
    }
  )
);

export default useSettingsStore;
