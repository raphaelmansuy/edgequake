/**
 * @module use-query-ui-store
 * @description Zustand store for query UI state management.
 * Manages streaming state, pending messages, and conversation filters.
 *
 * @implements FEAT0625 - Query streaming UI state
 * @implements FEAT0626 - Thinking indicator display
 * @implements FEAT0627 - Conversation filtering and search
 *
 * @enforces BR0615 - Streaming state transitions are ordered
 * @enforces BR0616 - Pending messages cleared on completion
 *
 * @see {@link use-query-store.ts} for query execution logic
 */
"use client";

import type { ConversationMode } from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

// ============================================================================
// Types
// ============================================================================

export type StreamingState =
  | "idle"
  | "thinking"
  | "generating"
  | "complete"
  | "error";

export interface PendingMessage {
  id: string;
  content: string;
  thinkingContent?: string;
  tokensGenerated: number;
  startTime: number;
  thinkingDuration?: number;
}

export interface ConversationFilters {
  mode: ConversationMode[] | null;
  archived: boolean;
  pinned: boolean | null;
  folderId: string | null;
  /** When true, filter for conversations without any folder (unfiled/root). */
  unfiled: boolean;
  search: string;
  dateFrom: string | null;
  dateTo: string | null;
}

export interface ConversationSort {
  field: "updated_at" | "created_at" | "title";
  order: "asc" | "desc";
}

// ============================================================================
// Store State
// ============================================================================

interface QueryUIState {
  // Active conversation
  activeConversationId: string | null;

  // Panel state
  historyPanelOpen: boolean;

  // Streaming state
  streamingState: StreamingState;
  pendingMessage: PendingMessage | null;
  abortController: AbortController | null;

  // Filters & sort
  filters: ConversationFilters;
  sort: ConversationSort;

  // Selection (for batch operations)
  selectedIds: Set<string>;
  isSelectionMode: boolean;
}

interface QueryUIActions {
  // Active conversation
  setActiveConversation: (id: string | null) => void;

  // Panel
  setHistoryPanelOpen: (open: boolean) => void;
  toggleHistoryPanel: () => void;

  // Streaming
  startStreaming: (conversationId: string) => AbortController;
  updateStreamingState: (state: StreamingState) => void;
  setPendingMessage: (message: PendingMessage | null) => void;
  appendToPendingMessage: (content: string) => void;
  setThinkingContent: (content: string) => void;
  completeStreaming: () => void;
  abortStreaming: () => void;

  // Filters
  setFilters: (filters: Partial<ConversationFilters>) => void;
  resetFilters: () => void;
  setSort: (sort: ConversationSort) => void;

  // Selection
  toggleSelection: (id: string) => void;
  selectAll: (ids: string[]) => void;
  clearSelection: () => void;
  setSelectionMode: (mode: boolean) => void;

  // Reset
  reset: () => void;
}

type QueryUIStore = QueryUIState & QueryUIActions;

// ============================================================================
// Default Values
// ============================================================================

const defaultFilters: ConversationFilters = {
  mode: null,
  archived: false,
  pinned: null,
  folderId: null,
  unfiled: true, // WHY: By default show unfiled conversations (not in any folder)
  search: "",
  dateFrom: null,
  dateTo: null,
};

const defaultSort: ConversationSort = {
  field: "updated_at",
  order: "desc",
};

const defaultState: QueryUIState = {
  activeConversationId: null,
  historyPanelOpen: true,
  streamingState: "idle",
  pendingMessage: null,
  abortController: null,
  filters: defaultFilters,
  sort: defaultSort,
  selectedIds: new Set(),
  isSelectionMode: false,
};

// ============================================================================
// Store Implementation
// ============================================================================

export const useQueryUIStore = create<QueryUIStore>()(
  persist(
    (set, get) => ({
      ...defaultState,

      // Active conversation
      setActiveConversation: (id) => {
        set({ activeConversationId: id });
      },

      // Panel
      setHistoryPanelOpen: (open) => set({ historyPanelOpen: open }),
      toggleHistoryPanel: () =>
        set((state) => ({ historyPanelOpen: !state.historyPanelOpen })),

      // Streaming
      startStreaming: (conversationId) => {
        const controller = new AbortController();
        set({
          activeConversationId: conversationId,
          streamingState: "thinking",
          abortController: controller,
          pendingMessage: {
            id: `pending-${Date.now()}`,
            content: "",
            tokensGenerated: 0,
            startTime: Date.now(),
          },
        });
        return controller;
      },

      updateStreamingState: (state) => set({ streamingState: state }),

      setPendingMessage: (message) => set({ pendingMessage: message }),

      appendToPendingMessage: (content) => {
        set((state) => {
          if (!state.pendingMessage) return state;
          return {
            pendingMessage: {
              ...state.pendingMessage,
              content: state.pendingMessage.content + content,
              tokensGenerated: state.pendingMessage.tokensGenerated + 1,
            },
            streamingState: "generating",
          };
        });
      },

      setThinkingContent: (content) => {
        set((state) => {
          if (!state.pendingMessage) return state;
          return {
            pendingMessage: {
              ...state.pendingMessage,
              thinkingContent: content,
              thinkingDuration: Date.now() - state.pendingMessage.startTime,
            },
          };
        });
      },

      completeStreaming: () => {
        set({
          streamingState: "complete",
          pendingMessage: null,
          abortController: null,
        });
      },

      abortStreaming: () => {
        const { abortController } = get();
        abortController?.abort();
        set({
          streamingState: "idle",
          pendingMessage: null,
          abortController: null,
        });
      },

      // Filters
      setFilters: (filters) =>
        set((state) => ({
          filters: { ...state.filters, ...filters },
        })),

      resetFilters: () => set({ filters: defaultFilters }),

      setSort: (sort) => set({ sort }),

      // Selection
      toggleSelection: (id) => {
        set((state) => {
          const newSet = new Set(state.selectedIds);
          if (newSet.has(id)) {
            newSet.delete(id);
          } else {
            newSet.add(id);
          }
          return { selectedIds: newSet };
        });
      },

      selectAll: (ids) => set({ selectedIds: new Set(ids) }),

      clearSelection: () =>
        set({ selectedIds: new Set(), isSelectionMode: false }),

      setSelectionMode: (mode) => set({ isSelectionMode: mode }),

      // Reset
      reset: () => set(defaultState),
    }),
    {
      name: "edgequake-query-ui",
      partialize: (state) => ({
        // Only persist these fields
        historyPanelOpen: state.historyPanelOpen,
        activeConversationId: state.activeConversationId,
        filters: state.filters,
        sort: state.sort,
      }),
    },
  ),
);

// ============================================================================
// Derived Selectors
// ============================================================================

export const useActiveConversationId = () =>
  useQueryUIStore((state) => state.activeConversationId);

export const useHistoryPanelOpen = () =>
  useQueryUIStore((state) => state.historyPanelOpen);

export const useStreamingState = () =>
  useQueryUIStore((state) => ({
    state: state.streamingState,
    pendingMessage: state.pendingMessage,
    isStreaming:
      state.streamingState !== "idle" && state.streamingState !== "complete",
  }));

export const useConversationFilters = () =>
  useQueryUIStore((state) => ({
    filters: state.filters,
    sort: state.sort,
    setFilters: state.setFilters,
    resetFilters: state.resetFilters,
    setSort: state.setSort,
  }));

export const useConversationSelection = () =>
  useQueryUIStore((state) => ({
    selectedIds: state.selectedIds,
    isSelectionMode: state.isSelectionMode,
    toggleSelection: state.toggleSelection,
    selectAll: state.selectAll,
    clearSelection: state.clearSelection,
    setSelectionMode: state.setSelectionMode,
    selectedCount: state.selectedIds.size,
  }));
