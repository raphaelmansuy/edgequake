/**
 * @module use-conversation-store
 * @description Zustand store for client-side conversation management.
 * Manages local conversation state, messages, and history panel.
 *
 * @implements UC0401 - User creates new conversation
 * @implements UC0405 - User views conversation history
 * @implements UC0406 - User renames conversation
 * @implements FEAT0403 - Local conversation storage
 * @implements FEAT0404 - Active conversation tracking
 *
 * @enforces BR0401 - Conversations persist in localStorage
 * @enforces BR0404 - Message order preserved
 * @enforces BR0405 - Streaming messages marked appropriately
 *
 * @see {@link docs/use_cases.md} UC0401, UC0405-0406
 */
"use client";

import { generateUUID } from "@/lib/utils/uuid";
import type { QueryContext, QueryMode } from "@/types";
import { create } from "zustand";
import { persist } from "zustand/middleware";

// Message type for conversation
export interface ConversationMessage {
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

// Conversation type
export interface Conversation {
  id: string;
  title: string;
  messages: ConversationMessage[];
  createdAt: number;
  updatedAt: number;
  tenantId?: string;
  workspaceId?: string;
}

interface ConversationState {
  // All conversations
  conversations: Conversation[];

  // Current active conversation ID
  activeConversationId: string | null;

  // Panel visibility
  historyPanelOpen: boolean;
}

interface ConversationActions {
  // Conversation management
  createConversation: (tenantId?: string, workspaceId?: string) => string;
  deleteConversation: (id: string) => void;
  setActiveConversation: (id: string | null) => void;
  renameConversation: (id: string, title: string) => void;
  clearConversations: () => void;

  // Message actions (operate on active conversation)
  addMessage: (message: ConversationMessage) => void;
  updateMessage: (
    messageId: string,
    updates: Partial<ConversationMessage>
  ) => void;
  clearActiveConversation: () => void;

  // Get current conversation
  getActiveConversation: () => Conversation | null;

  // Panel toggle
  setHistoryPanelOpen: (open: boolean) => void;
  toggleHistoryPanel: () => void;

  // Auto-title based on first message
  autoTitleConversation: (conversationId: string) => void;
}

type ConversationStore = ConversationState & ConversationActions;

const generateConversationTitle = (firstMessage: string): string => {
  // Truncate to first 50 characters
  const truncated = firstMessage.slice(0, 50);
  return truncated.length < firstMessage.length ? truncated + "..." : truncated;
};

const createNewConversation = (
  tenantId?: string,
  workspaceId?: string
): Conversation => ({
  id: generateUUID(),
  title: `Chat ${new Date().toLocaleDateString()}`,
  messages: [],
  createdAt: Date.now(),
  updatedAt: Date.now(),
  tenantId,
  workspaceId,
});

export const useConversationStore = create<ConversationStore>()(
  persist(
    (set, get) => ({
      conversations: [],
      activeConversationId: null,
      historyPanelOpen: true,

      // Conversation management
      createConversation: (tenantId?: string, workspaceId?: string) => {
        const newConversation = createNewConversation(tenantId, workspaceId);
        set((state) => ({
          conversations: [newConversation, ...state.conversations],
          activeConversationId: newConversation.id,
        }));
        return newConversation.id;
      },

      deleteConversation: (id) => {
        set((state) => {
          const newConversations = state.conversations.filter(
            (c) => c.id !== id
          );
          const newActiveId =
            state.activeConversationId === id
              ? newConversations[0]?.id ?? null
              : state.activeConversationId;
          return {
            conversations: newConversations,
            activeConversationId: newActiveId,
          };
        });
      },

      setActiveConversation: (id) => {
        set({ activeConversationId: id });
      },

      renameConversation: (id, title) => {
        set((state) => ({
          conversations: state.conversations.map((c) =>
            c.id === id ? { ...c, title, updatedAt: Date.now() } : c
          ),
        }));
      },

      clearConversations: () => {
        set({ conversations: [], activeConversationId: null });
      },

      // Message actions
      addMessage: (message) => {
        set((state) => {
          let { activeConversationId } = state;
          let conversations = [...state.conversations];

          // If no active conversation, create one
          if (!activeConversationId) {
            const newConversation = createNewConversation();
            conversations = [newConversation, ...conversations];
            activeConversationId = newConversation.id;
          }

          return {
            conversations: conversations.map((c) =>
              c.id === activeConversationId
                ? {
                    ...c,
                    messages: [...c.messages, message],
                    updatedAt: Date.now(),
                  }
                : c
            ),
            activeConversationId,
          };
        });
      },

      updateMessage: (messageId, updates) => {
        set((state) => ({
          conversations: state.conversations.map((c) =>
            c.id === state.activeConversationId
              ? {
                  ...c,
                  messages: c.messages.map((m) =>
                    m.id === messageId ? { ...m, ...updates } : m
                  ),
                  updatedAt: Date.now(),
                }
              : c
          ),
        }));
      },

      clearActiveConversation: () => {
        set((state) => ({
          conversations: state.conversations.map((c) =>
            c.id === state.activeConversationId
              ? { ...c, messages: [], updatedAt: Date.now() }
              : c
          ),
        }));
      },

      getActiveConversation: () => {
        const state = get();
        return (
          state.conversations.find(
            (c) => c.id === state.activeConversationId
          ) ?? null
        );
      },

      // Panel toggle
      setHistoryPanelOpen: (open) => set({ historyPanelOpen: open }),
      toggleHistoryPanel: () =>
        set((state) => ({ historyPanelOpen: !state.historyPanelOpen })),

      // Auto-title based on first message
      autoTitleConversation: (conversationId) => {
        set((state) => {
          const conversation = state.conversations.find(
            (c) => c.id === conversationId
          );
          if (!conversation) return state;

          const firstUserMessage = conversation.messages.find(
            (m) => m.role === "user"
          );
          if (!firstUserMessage) return state;

          const title = generateConversationTitle(firstUserMessage.content);

          return {
            conversations: state.conversations.map((c) =>
              c.id === conversationId
                ? { ...c, title, updatedAt: Date.now() }
                : c
            ),
          };
        });
      },
    }),
    {
      name: "edgequake-conversations",
      partialize: (state) => ({
        conversations: state.conversations,
        activeConversationId: state.activeConversationId,
        historyPanelOpen: state.historyPanelOpen,
      }),
    }
  )
);

// Selectors
export const useActiveConversation = () => {
  const store = useConversationStore();
  return (
    store.conversations.find((c) => c.id === store.activeConversationId) ?? null
  );
};

export const useActiveMessages = () => {
  const conversation = useActiveConversation();
  return conversation?.messages ?? [];
};

export const useConversationList = (limit?: number) => {
  const { conversations } = useConversationStore();
  // Sort by updatedAt (most recent first)
  const sorted = [...conversations].sort((a, b) => b.updatedAt - a.updatedAt);
  return limit ? sorted.slice(0, limit) : sorted;
};

export default useConversationStore;
