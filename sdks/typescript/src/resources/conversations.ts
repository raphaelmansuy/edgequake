/**
 * Conversations resource — conversation management with messages sub-resource.
 *
 * WHY: Updated to use cursor-based pagination matching Rust API.
 * @module resources/conversations
 * @see edgequake/crates/edgequake-api/src/handlers/conversations.rs
 */

import type { HttpTransport } from "../transport/types.js";
import type { BulkOperationResponse } from "../types/common.js";
import type {
  BulkArchiveRequest,
  BulkDeleteRequest,
  BulkMoveRequest,
  ConversationDetail,
  ConversationInfo,
  CreateConversationRequest,
  CreateMessageRequest,
  ImportConversationsRequest,
  ImportConversationsResponse,
  ListConversationsQuery,
  ListMessagesQuery,
  MessageInfo,
  PaginatedConversationsResponse,
  PaginatedMessagesResponse,
  ShareResponse,
  UpdateConversationRequest,
  UpdateMessageRequest,
} from "../types/conversations.js";
import { Resource } from "./base.js";

/** Messages sub-resource accessed via `client.conversations.messages`. */
export class MessagesResource extends Resource {
  /**
   * List messages in a conversation (cursor-based pagination).
   * WHY: Rust returns PaginatedMessagesResponse { items, pagination }.
   */
  async list(
    conversationId: string,
    query?: ListMessagesQuery,
  ): Promise<PaginatedMessagesResponse> {
    const params = new URLSearchParams();
    if (query?.cursor) params.set("cursor", query.cursor);
    if (query?.limit) params.set("limit", String(query.limit));
    const qs = params.toString();
    const path = `/api/v1/conversations/${conversationId}/messages${qs ? `?${qs}` : ""}`;
    return this._get(path);
  }

  /** Add a message to a conversation. */
  async create(
    conversationId: string,
    request: CreateMessageRequest,
  ): Promise<MessageInfo> {
    return this._post(
      `/api/v1/conversations/${conversationId}/messages`,
      request,
    );
  }

  /** Update a message (feedback, content edit). */
  async update(
    messageId: string,
    request: UpdateMessageRequest,
  ): Promise<MessageInfo> {
    return this._patch(`/api/v1/messages/${messageId}`, request);
  }

  /** Delete a message. */
  async delete(messageId: string): Promise<void> {
    await this._del(`/api/v1/messages/${messageId}`);
  }
}

/** Conversations resource with messages sub-namespace. */
export class ConversationsResource extends Resource {
  /** Messages sub-resource. */
  readonly messages: MessagesResource;

  constructor(transport: HttpTransport) {
    super(transport);
    this.messages = new MessagesResource(transport);
  }

  /**
   * List conversations with cursor-based pagination and filters.
   * WHY: Rust uses cursor + filter[key] bracket params, not offset pagination.
   */
  async list(
    query?: ListConversationsQuery,
  ): Promise<PaginatedConversationsResponse> {
    const params = new URLSearchParams();
    if (query?.cursor) params.set("cursor", query.cursor);
    if (query?.limit) params.set("limit", String(query.limit));
    if (query?.filter_mode) params.set("filter[mode]", query.filter_mode);
    if (query?.filter_archived !== undefined)
      params.set("filter[archived]", String(query.filter_archived));
    if (query?.filter_pinned !== undefined)
      params.set("filter[pinned]", String(query.filter_pinned));
    if (query?.filter_folder_id)
      params.set("filter[folder_id]", query.filter_folder_id);
    if (query?.filter_search) params.set("filter[search]", query.filter_search);
    if (query?.sort) params.set("sort", query.sort);
    if (query?.order) params.set("order", query.order);
    const qs = params.toString();
    const path = `/api/v1/conversations${qs ? `?${qs}` : ""}`;
    return this._get(path);
  }

  /**
   * Get conversation details including messages.
   * WHY: Rust returns ConversationWithMessagesResponse { conversation, messages }.
   */
  async get(id: string): Promise<ConversationDetail> {
    return this._get(`/api/v1/conversations/${id}`);
  }

  /** Create a new conversation. */
  async create(request: CreateConversationRequest): Promise<ConversationInfo> {
    return this._post("/api/v1/conversations", request);
  }

  /** Update conversation metadata (title, folder, pin, archive). */
  async update(
    id: string,
    request: UpdateConversationRequest,
  ): Promise<ConversationInfo> {
    return this._patch(`/api/v1/conversations/${id}`, request);
  }

  /** Delete a conversation. */
  async delete(id: string): Promise<void> {
    await this._del(`/api/v1/conversations/${id}`);
  }

  /** Share a conversation via public link. */
  async share(id: string): Promise<ShareResponse> {
    return this._post(`/api/v1/conversations/${id}/share`);
  }

  /** Revoke share link for a conversation. */
  async unshare(id: string): Promise<void> {
    await this._del(`/api/v1/conversations/${id}/share`);
  }

  /** Import conversations (e.g., from ChatGPT export). */
  async import(
    request: ImportConversationsRequest,
  ): Promise<ImportConversationsResponse> {
    return this._post("/api/v1/conversations/import", request);
  }

  /** Bulk delete conversations. */
  async bulkDelete(request: BulkDeleteRequest): Promise<BulkOperationResponse> {
    return this._post("/api/v1/conversations/bulk/delete", request);
  }

  /** Bulk archive conversations. */
  async bulkArchive(
    request: BulkArchiveRequest,
  ): Promise<BulkOperationResponse> {
    return this._post("/api/v1/conversations/bulk/archive", request);
  }

  /** Bulk move conversations to folder. */
  async bulkMove(request: BulkMoveRequest): Promise<BulkOperationResponse> {
    return this._post("/api/v1/conversations/bulk/move", request);
  }
}
