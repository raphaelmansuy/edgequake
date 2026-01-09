/**
 * Query Store - Manages query execution state and conversation history.
 *
 * @implements UC0201 - Execute Query
 * @implements UC0202 - Query with Conversation History
 * @implements UC0203 - Stream Query Response
 * @implements FEAT0007 - Multi-Mode Query Execution (mode selection)
 * @implements FEAT0601 - Knowledge Graph Visualization (context display)
 *
 * @enforces BR0104 - Conversation history included in context
 * @enforces BR0105 - Empty queries are rejected (via UI validation)
 *
 * @description
 * This Zustand store manages:
 * - Current query input and execution state
 * - Conversation message history (persisted to localStorage)
 * - Streaming response accumulation
 * - Query history with favorites
 *
 * @see useQueryUIStore for UI-specific query state
 * @see useConversationStore for backend conversation sync
 */

"use client";

import { generateUUID } from "@/lib/utils/uuid";
import type { QueryContext, QueryHistoryItem, QueryMode } from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

/**
 * Message type for conversation.
 *
 * @implements UC0202 - Conversation messages with history
 */
export interface ChatMessage {
  id: string;
  role: "user" | "assistant";
  content: string;
  mode?: QueryMode;
  tokensUsed?: number;
  durationMs?: number;
  thinkingTimeMs?: number;
  context?: QueryContext;
  isError?: boolean;
  isStreaming?: boolean;
  timestamp?: number;
}

interface QueryState {
  // Current query
  currentQuery: string;
  isQuerying: boolean;

  // Conversation messages (persisted)
  conversationMessages: ChatMessage[];

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

  // Conversation actions
  setConversationMessages: (messages: ChatMessage[]) => void;
  addConversationMessage: (message: ChatMessage) => void;
  updateConversationMessage: (
    id: string,
    updates: Partial<ChatMessage>
  ) => void;
  clearConversation: () => void;

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
  conversationMessages: [],
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

      // Conversation actions
      setConversationMessages: (messages) =>
        set({ conversationMessages: messages }),
      addConversationMessage: (message) =>
        set((state) => ({
          conversationMessages: [...state.conversationMessages, message],
        })),
      updateConversationMessage: (id, updates) =>
        set((state) => ({
          conversationMessages: state.conversationMessages.map((msg) =>
            msg.id === id ? { ...msg, ...updates } : msg
          ),
        })),
      clearConversation: () => set({ conversationMessages: [] }),

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
              id: generateUUID(),
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
        conversationMessages: state.conversationMessages,
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
