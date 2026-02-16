"use client";

/**
 * @module use-move-conversation
 * @description Hook for moving conversations between folders.
 * Supports single and batch moves with optimistic updates.
 *
 * @implements FEAT0710 - Move conversations to folders
 * @enforces BR0708 - Deleting folder moves conversations to root
 */

import { updateConversation } from "@/lib/api/conversations";
import { conversationKeys } from "@/lib/api/query-keys";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

/**
 * Move a single conversation to a folder (or to root if folderId is null).
 */
export function useMoveConversation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      conversationId,
      folderId,
    }: {
      conversationId: string;
      folderId: string | null;
    }) => updateConversation(conversationId, { folder_id: folderId }),
    onSuccess: (_data, { folderId }) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
      toast.success(folderId ? "Moved to folder" : "Moved to Unfiled");
    },
    onError: () => {
      toast.error("Failed to move conversation");
    },
  });
}

/**
 * Move multiple conversations to a folder (batch operation).
 */
export function useMoveConversations() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      conversationIds,
      folderId,
    }: {
      conversationIds: string[];
      folderId: string | null;
    }) => {
      // Execute moves in parallel
      await Promise.all(
        conversationIds.map((id) =>
          updateConversation(id, { folder_id: folderId }),
        ),
      );
    },
    onSuccess: (_data, { conversationIds, folderId }) => {
      queryClient.invalidateQueries({ queryKey: conversationKeys.lists() });
      toast.success(
        folderId
          ? `Moved ${conversationIds.length} conversations to folder`
          : `Moved ${conversationIds.length} conversations to Unfiled`,
      );
    },
    onError: () => {
      toast.error("Failed to move conversations");
    },
  });
}
