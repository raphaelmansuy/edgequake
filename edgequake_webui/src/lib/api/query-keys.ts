import type { ConversationFilterParams } from "@/types";

/**
 * Query keys for React Query caching.
 * Follows the factory pattern for hierarchical key structure.
 */

export const conversationKeys = {
  all: ["conversations"] as const,
  lists: () => [...conversationKeys.all, "list"] as const,
  list: (filters: ConversationFilterParams | Record<string, unknown>) =>
    [...conversationKeys.lists(), filters] as const,
  details: () => [...conversationKeys.all, "detail"] as const,
  detail: (id: string) => [...conversationKeys.details(), id] as const,
  messages: (id: string) =>
    [...conversationKeys.detail(id), "messages"] as const,
};

export const folderKeys = {
  all: ["folders"] as const,
  list: () => [...folderKeys.all, "list"] as const,
};
