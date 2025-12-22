"use client";

import type { QueryHistoryItem } from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

interface QueryState {
  // Current query
  currentQuery: string;
  isQuerying: boolean;

  // Streaming response
  streamingResponse: string;
  isStreaming: boolean;

  // History
  history: QueryHistoryItem[];

  // Error
  error: string | null;
}

interface QueryActions {
  // Query actions
  setCurrentQuery: (query: string) => void;
  setIsQuerying: (isQuerying: boolean) => void;

  // Streaming actions
  appendStreamChunk: (chunk: string) => void;
  clearStreamingResponse: () => void;
  setIsStreaming: (isStreaming: boolean) => void;

  // History actions
  addToHistory: (item: Omit<QueryHistoryItem, "id" | "timestamp">) => void;
  toggleFavorite: (id: string) => void;
  removeFromHistory: (id: string) => void;
  clearHistory: () => void;

  // Error
  setError: (error: string | null) => void;

  // Reset
  reset: () => void;
}

type QueryStore = QueryState & QueryActions;

const initialState: QueryState = {
  currentQuery: "",
  isQuerying: false,
  streamingResponse: "",
  isStreaming: false,
  history: [],
  error: null,
};

export const useQueryStore = create<QueryStore>()(
  persist(
    (set) => ({
      ...initialState,

      // Query actions
      setCurrentQuery: (query) => set({ currentQuery: query }),
      setIsQuerying: (isQuerying) => set({ isQuerying }),

      // Streaming actions
      appendStreamChunk: (chunk) =>
        set((state) => ({
          streamingResponse: state.streamingResponse + chunk,
        })),

      clearStreamingResponse: () => set({ streamingResponse: "" }),

      setIsStreaming: (isStreaming) => set({ isStreaming }),

      // History actions
      addToHistory: (item) =>
        set((state) => ({
          history: [
            {
              ...item,
              id: crypto.randomUUID(),
              timestamp: new Date().toISOString(),
            },
            ...state.history.slice(0, 99), // Keep last 100 items
          ],
        })),

      toggleFavorite: (id) =>
        set((state) => ({
          history: state.history.map((item) =>
            item.id === id ? { ...item, isFavorite: !item.isFavorite } : item
          ),
        })),

      removeFromHistory: (id) =>
        set((state) => ({
          history: state.history.filter((item) => item.id !== id),
        })),

      clearHistory: () => set({ history: [] }),

      // Error
      setError: (error) => set({ error }),

      // Reset
      reset: () => set(initialState),
    }),
    {
      name: "edgequake-query",
      partialize: (state) => ({
        history: state.history,
      }),
    }
  )
);

// Selectors
export const useFavoriteQueries = () => {
  const { history } = useQueryStore();
  return history.filter((item) => item.isFavorite);
};

export const useRecentQueries = (limit = 10) => {
  const { history } = useQueryStore();
  return history.slice(0, limit);
};

export default useQueryStore;
