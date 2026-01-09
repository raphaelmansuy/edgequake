/**
 * @module query-keys
 * @description Query keys for React Query caching.
 * Follows the factory pattern for hierarchical key structure.
 *
 * @implements FEAT0711 - Hierarchical query key structure
 * @implements FEAT0712 - Automatic cache invalidation
 *
 * @enforces BR0709 - Keys include all filter parameters
 * @enforces BR0710 - Detail keys nested under list keys
 */

import type { ConversationFilterParams } from "@/types";

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
