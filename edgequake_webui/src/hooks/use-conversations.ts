/**
 * @module use-conversations
 * @description React Query hooks for conversation management.
 * Provides CRUD operations and real-time updates for chat conversations.
 *
 * @implements UC0401 - User creates new conversation
 * @implements UC0402 - User views conversation history
 * @implements UC0403 - User shares conversation with link
 * @implements UC0404 - User imports/exports conversations
 * @implements FEAT0401 - Conversation persistence
 * @implements FEAT0402 - Infinite scroll pagination
 *
 * @enforces BR0401 - Conversations persist across sessions
 * @enforces BR0403 - Shared links expire after TTL
 *
 * @see {@link docs/use_cases.md} UC0401-0404
 */
"use client";

import {
  createConversation,
  createMessage,
  deleteConversation,
  deleteConversations,
  getConversation,
  importConversations,
  listConversations,
  shareConversation,
  unshareConversation,
  updateConversation,
  updateMessage,
} from "@/lib/api/conversations";
import { conversationKeys } from "@/lib/api/query-keys";
import type {
  ConversationFilterParams,
  ConversationWithMessages,
  CreateConversationRequest,
  CreateMessageRequest,
  ImportConversationsRequest,
  ServerMessage,
  UpdateConversationRequest,
  UpdateMessageRequest,
} from "@/types";
import {
  useInfiniteQuery,
  useMutation,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { toast } from "sonner";

// ============================================================================
// List Conversations (Infinite Query)
// ============================================================================

export function useConversations(filters?: ConversationFilterParams) {
  return useInfiniteQuery({
    queryKey: conversationKeys.list(filters ?? {}),
    queryFn: async ({ pageParam }) => {
      return listConversations({
        cursor: pageParam as string | undefined,
        limit: 20,
        ...filters,
      });
    },
    initialPageParam: undefined as string | undefined,
    getNextPageParam: (lastPage) =>
      lastPage.pagination.has_more
        ? lastPage.pagination.next_cursor
        : undefined,
    staleTime: 30_000, // 30 seconds
  });
}

// ============================================================================
// Single Conversation
// ============================================================================

export function useConversation(conversationId: string | null) {
  return useQuery({
    queryKey: conversationKeys.detail(conversationId ?? ""),
    queryFn: () => getConversation(conversationId!),
    enabled: !!conversationId,
    staleTime: 60_000, // 1 minute
  });
}

// ============================================================================
// Mutations
// ============================================================================

export function useCreateConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateConversationRequest) => createConversation(data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
    },
  });
}

export function useUpdateConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      id,
      data,
    }: {
      id: string;
      data: UpdateConversationRequest;
    }) => updateConversation(id, data),
    onMutate: async ({ id, data }) => {
      // Optimistic update
      await queryClient.cancelQueries({
        queryKey: conversationKeys.detail(id),
      });

      const previousConversation =
        queryClient.getQueryData<ConversationWithMessages>(
          conversationKeys.detail(id)
        );

      if (previousConversation) {
        queryClient.setQueryData(conversationKeys.detail(id), {
          ...previousConversation,
          ...data,
        });
      }

      return { previousConversation };
    },
    onError: (_err, { id }, context) => {
      if (context?.previousConversation) {
        queryClient.setQueryData(
          conversationKeys.detail(id),
          context.previousConversation
        );
      }
      toast.error("Failed to update conversation");
    },
    onSettled: (_, __, { id }) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
    },
  });
}

export function useDeleteConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => deleteConversation(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
      toast.success("Conversation deleted");
    },
    onError: () => {
      toast.error("Failed to delete conversation");
    },
  });
}

export function useDeleteConversations() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (ids: string[]) => deleteConversations(ids),
    onSuccess: (_, ids) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
      toast.success(`Deleted ${ids.length} conversations`);
    },
    onError: () => {
      toast.error("Failed to delete conversations");
    },
  });
}

// ============================================================================
// Message Mutations
// ============================================================================

export function useCreateMessage(conversationId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateMessageRequest) =>
      createMessage(conversationId, data),
    onMutate: async (data) => {
      // Optimistic: add user message immediately
      const optimisticMessage: ServerMessage = {
        id: `temp-${Date.now()}`,
        conversation_id: conversationId,
        role: "user",
        content: data.content,
        is_error: false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      await queryClient.cancelQueries({
        queryKey: conversationKeys.detail(conversationId),
      });

      const previous = queryClient.getQueryData<ConversationWithMessages>(
        conversationKeys.detail(conversationId)
      );

      if (previous) {
        queryClient.setQueryData(conversationKeys.detail(conversationId), {
          ...previous,
          messages: [...previous.messages, optimisticMessage],
        });
      }

      return { previous, optimisticMessage };
    },
    onError: (_err, _, context) => {
      if (context?.previous) {
        queryClient.setQueryData(
          conversationKeys.detail(conversationId),
          context.previous
        );
      }
      toast.error("Failed to send message");
    },
    onSettled: () => {
      queryClient.invalidateQueries({
        queryKey: conversationKeys.detail(conversationId),
      });
    },
  });
}

export function useUpdateMessage(conversationId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      messageId,
      data,
    }: {
      messageId: string;
      data: UpdateMessageRequest;
    }) => updateMessage(conversationId, messageId, data),
    onSettled: () => {
      queryClient.invalidateQueries({
        queryKey: conversationKeys.detail(conversationId),
      });
    },
  });
}

// ============================================================================
// Sharing
// ============================================================================

export function useShareConversation() {
  return useMutation({
    mutationFn: (id: string) => shareConversation(id),
    onSuccess: (data) => {
      navigator.clipboard.writeText(data.share_url);
      toast.success("Share link copied to clipboard");
    },
    onError: () => {
      toast.error("Failed to generate share link");
    },
  });
}

export function useUnshareConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => unshareConversation(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.detail(id) });
      toast.success("Share link removed");
    },
  });
}

// ============================================================================
// Import
// ============================================================================

export function useImportConversations() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: ImportConversationsRequest) => importConversations(data),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
      toast.success(
        `Imported ${result.imported} conversations${
          result.failed > 0 ? `, ${result.failed} failed` : ""
        }`
      );
    },
    onError: () => {
      toast.error("Failed to import conversations");
    },
  });
}
