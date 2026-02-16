/**
 * @module conversations-api
 * @description Conversations API client.
 * Provides CRUD operations for conversations and messages.
 *
 * @implements FEAT0706 - Conversation list with pagination
 * @implements FEAT0707 - Message history retrieval
 * @implements FEAT0708 - Conversation sharing
 *
 * @enforces BR0705 - Cursor pagination for large lists
 * @enforces BR0706 - Messages ordered by created_at
 */

import type {
  ConversationFilterParams,
  ConversationWithMessages,
  CreateConversationRequest,
  CreateMessageRequest,
  CursorPaginationParams,
  ImportConversationsRequest,
  ImportConversationsResponse,
  PaginatedConversations,
  PaginatedMessages,
  ServerConversation,
  ServerMessage,
  ShareConversationResponse,
  UpdateConversationRequest,
  UpdateMessageRequest,
} from "@/types";
import { api } from "./client";

// ============================================================================
// Conversations
// ============================================================================

/**
 * List conversations with pagination and filtering.
 */
export async function listConversations(
  params?: CursorPaginationParams & ConversationFilterParams,
): Promise<PaginatedConversations> {
  const searchParams = new URLSearchParams();

  if (params?.cursor) searchParams.set("cursor", params.cursor);
  if (params?.limit) searchParams.set("limit", String(params.limit));
  if (params?.mode?.length) {
    params.mode.forEach((m) => searchParams.append("filter[mode]", m));
  }
  if (params?.archived !== undefined) {
    searchParams.set("filter[archived]", String(params.archived));
  }
  if (params?.pinned !== undefined) {
    searchParams.set("filter[pinned]", String(params.pinned));
  }
  if (params?.folder_id)
    searchParams.set("filter[folder_id]", params.folder_id);
  if (params?.unfiled !== undefined)
    searchParams.set("filter[unfiled]", String(params.unfiled));
  if (params?.search) searchParams.set("filter[search]", params.search);
  if (params?.date_from)
    searchParams.set("filter[date_from]", params.date_from);
  if (params?.date_to) searchParams.set("filter[date_to]", params.date_to);
  if (params?.sort) searchParams.set("sort", params.sort);
  if (params?.order) searchParams.set("order", params.order);

  const query = searchParams.toString();
  return api.get<PaginatedConversations>(
    `/conversations${query ? `?${query}` : ""}`,
  );
}

/**
 * Get a single conversation by ID (includes messages).
 */
export async function getConversation(
  conversationId: string,
): Promise<ConversationWithMessages> {
  return api.get<ConversationWithMessages>(`/conversations/${conversationId}`);
}

/**
 * Create a new conversation.
 */
export async function createConversation(
  data: CreateConversationRequest,
): Promise<ServerConversation> {
  return api.post<ServerConversation>("/conversations", data);
}

/**
 * Update a conversation.
 */
export async function updateConversation(
  conversationId: string,
  data: UpdateConversationRequest,
): Promise<ServerConversation> {
  return api.patch<ServerConversation>(
    `/conversations/${conversationId}`,
    data,
  );
}

/**
 * Delete a conversation.
 */
export async function deleteConversation(
  conversationId: string,
): Promise<void> {
  return api.delete(`/conversations/${conversationId}`);
}

/**
 * Batch delete conversations.
 */
export async function deleteConversations(ids: string[]): Promise<void> {
  return api.post("/conversations/batch-delete", { ids });
}

// ============================================================================
// Messages
// ============================================================================

/**
 * List messages in a conversation.
 */
export async function listMessages(
  conversationId: string,
  params?: CursorPaginationParams,
): Promise<PaginatedMessages> {
  const searchParams = new URLSearchParams();
  if (params?.cursor) searchParams.set("cursor", params.cursor);
  if (params?.limit) searchParams.set("limit", String(params.limit));

  const query = searchParams.toString();
  return api.get<PaginatedMessages>(
    `/conversations/${conversationId}/messages${query ? `?${query}` : ""}`,
  );
}

/**
 * Add a message to a conversation.
 * Returns the user message immediately; AI response comes via streaming.
 */
export async function createMessage(
  conversationId: string,
  data: CreateMessageRequest,
): Promise<ServerMessage> {
  return api.post<ServerMessage>(
    `/conversations/${conversationId}/messages`,
    data,
  );
}

/**
 * Update a message (e.g., after streaming completes).
 */
export async function updateMessage(
  conversationId: string,
  messageId: string,
  data: UpdateMessageRequest,
): Promise<ServerMessage> {
  return api.patch<ServerMessage>(
    `/conversations/${conversationId}/messages/${messageId}`,
    data,
  );
}

/**
 * Delete a message from a conversation.
 * Used for regeneration - removes old assistant response before generating new one.
 */
export async function deleteMessage(messageId: string): Promise<void> {
  return api.delete(`/messages/${messageId}`);
}

// ============================================================================
// Sharing
// ============================================================================

/**
 * Generate a shareable link for a conversation.
 */
export async function shareConversation(
  conversationId: string,
): Promise<ShareConversationResponse> {
  return api.post<ShareConversationResponse>(
    `/conversations/${conversationId}/share`,
  );
}

/**
 * Remove the shareable link from a conversation.
 */
export async function unshareConversation(
  conversationId: string,
): Promise<void> {
  return api.delete(`/conversations/${conversationId}/share`);
}

// ============================================================================
// Import
// ============================================================================

/**
 * Import conversations from localStorage.
 */
export async function importConversations(
  data: ImportConversationsRequest,
): Promise<ImportConversationsResponse> {
  return api.post<ImportConversationsResponse>("/conversations/import", data);
}
