"use client";

/**
 * @module use-query-page-state
 * @description Hook for managing query page state.
 * Coordinates UI store with conversation data and streaming.
 *
 * @implements FEAT0645 - Query page state coordination
 * @implements FEAT0646 - Conversation and streaming integration
 *
 * @enforces BR0630 - State synced with active conversation
 * @enforces BR0631 - Streaming state resets on conversation change
 */

import {
  useActiveConversationId,
  useQueryUIStore,
  useStreamingState,
} from "@/stores/use-query-ui-store";
import type {
  ConversationWithMessages,
  ServerConversation,
  ServerMessage,
} from "@/types";
import { useCallback, useMemo } from "react";
import {
  useConversation,
  useConversations,
  useCreateConversation,
  useCreateMessage,
} from "./use-conversations";

interface QueryPageState {
  // Current conversation
  conversation: ConversationWithMessages | null;
  isLoadingConversation: boolean;

  // Messages (including pending)
  messages: ServerMessage[];

  // Streaming
  isStreaming: boolean;
  streamingState: ReturnType<typeof useStreamingState>;

  // Conversation list
  conversations: ServerConversation[];
  isLoadingList: boolean;
  hasMoreConversations: boolean;
  loadMoreConversations: () => void;

  // Actions
  createNewConversation: () => Promise<string>;
  sendMessage: (content: string) => Promise<void>;
  switchConversation: (id: string) => void;

  // Panel state
  historyPanelOpen: boolean;
  toggleHistoryPanel: () => void;
}

export function useQueryPageState(): QueryPageState {
  const store = useQueryUIStore();
  const activeId = useActiveConversationId();
  const streamingState = useStreamingState();

  // Server state
  const { data: conversationData, isLoading: isLoadingConversation } =
    useConversation(activeId);

  const {
    data: conversationsData,
    isLoading: isLoadingList,
    fetchNextPage,
    hasNextPage,
  } = useConversations({
    archived: store.filters.archived,
    mode: store.filters.mode ?? undefined,
    pinned: store.filters.pinned ?? undefined,
    folder_id: store.filters.folderId ?? undefined,
    unfiled: store.filters.unfiled || undefined,
    search: store.filters.search || undefined,
    date_from: store.filters.dateFrom ?? undefined,
    date_to: store.filters.dateTo ?? undefined,
    sort: store.sort.field,
    order: store.sort.order,
  });

  // Mutations
  const createConversationMutation = useCreateConversation();
  const createMessageMutation = useCreateMessage(activeId ?? "");

  // Flatten paginated conversations
  const conversations = useMemo(() => {
    return conversationsData?.pages.flatMap((page) => page.items) ?? [];
  }, [conversationsData]);

  // Combine real messages with pending message
  const messages = useMemo(() => {
    const realMessages = conversationData?.messages ?? [];

    if (streamingState.pendingMessage && streamingState.isStreaming) {
      // Add pending assistant message
      const pendingAssistantMessage: ServerMessage = {
        id: streamingState.pendingMessage.id,
        conversation_id: activeId ?? "",
        role: "assistant",
        content: streamingState.pendingMessage.content,
        is_error: false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        context: streamingState.pendingMessage.thinkingContent
          ? { thinking: streamingState.pendingMessage.thinkingContent }
          : undefined,
      };
      return [...realMessages, pendingAssistantMessage];
    }

    return realMessages;
  }, [conversationData?.messages, streamingState, activeId]);

  // Actions
  const createNewConversation = useCallback(async () => {
    const conversation = await createConversationMutation.mutateAsync({
      mode: "hybrid",
    });
    store.setActiveConversation(conversation.id);
    return conversation.id;
  }, [createConversationMutation, store]);

  const sendMessage = useCallback(
    async (content: string) => {
      let targetId = activeId;

      if (!targetId) {
        // Create conversation first
        targetId = await createNewConversation();
      }

      await createMessageMutation.mutateAsync({
        content,
        role: "user",
        stream: true,
      });
    },
    [activeId, createNewConversation, createMessageMutation],
  );

  const switchConversation = useCallback(
    (id: string) => {
      if (streamingState.isStreaming) {
        store.abortStreaming();
      }
      store.setActiveConversation(id);
    },
    [store, streamingState.isStreaming],
  );

  return {
    conversation: conversationData ?? null,
    isLoadingConversation,
    messages,
    isStreaming: streamingState.isStreaming,
    streamingState,
    conversations,
    isLoadingList,
    hasMoreConversations: hasNextPage ?? false,
    loadMoreConversations: fetchNextPage,
    createNewConversation,
    sendMessage,
    switchConversation,
    historyPanelOpen: store.historyPanelOpen,
    toggleHistoryPanel: store.toggleHistoryPanel,
  };
}
