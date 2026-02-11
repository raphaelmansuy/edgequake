/**
 * Conversations resource — conversation management with messages sub-resource.
 *
 * @module resources/conversations
 * @see edgequake/crates/edgequake-api/src/handlers/conversations.rs
 */

import { Resource } from "./base.js";
import { Paginator } from "../pagination.js";
import type { Page, BulkOperationResponse } from "../types/common.js";
import type {
  ConversationInfo,
  ConversationDetail,
  CreateConversationRequest,
  UpdateConversationRequest,
  ListConversationsQuery,
  MessageInfo,
  CreateMessageRequest,
  UpdateMessageRequest,
  ShareResponse,
  ImportConversationsRequest,
  ImportConversationsResponse,
  BulkDeleteRequest,
  BulkArchiveRequest,
  BulkMoveRequest,
} from "../types/conversations.js";
import type { HttpTransport } from "../transport/types.js";

/** Messages sub-resource accessed via `client.conversations.messages`. */
export class MessagesResource extends Resource {
  /** List messages in a conversation. */
  async list(conversationId: string): Promise<MessageInfo[]> {
    return this._get(`/api/v1/conversations/${conversationId}/messages`);
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

  /** List conversations with optional filters + pagination. */
  list(query?: ListConversationsQuery): Paginator<ConversationInfo> {
    return new Paginator(
      async (page, perPage) => {
        const params = new URLSearchParams();
        params.set("page", String(page));
        params.set("per_page", String(perPage));
        if (query?.folder_id) params.set("folder_id", query.folder_id);
        if (query?.search) params.set("search", query.search);
        if (query?.archived !== undefined)
          params.set("archived", String(query.archived));
        const path = `/api/v1/conversations?${params}`;
        return this._get<Page<ConversationInfo>>(path);
      },
      query?.limit ?? 20,
    );
  }

  /** Get conversation details including messages. */
  async get(id: string): Promise<ConversationDetail> {
    return this._get(`/api/v1/conversations/${id}`);
  }

  /** Create a new conversation. */
  async create(
    request: CreateConversationRequest,
  ): Promise<ConversationInfo> {
    return this._post("/api/v1/conversations", request);
  }

  /** Update conversation metadata (title, folder, pin). */
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
  async bulkDelete(
    request: BulkDeleteRequest,
  ): Promise<BulkOperationResponse> {
    return this._post("/api/v1/conversations/bulk/delete", request);
  }

  /** Bulk archive conversations. */
  async bulkArchive(
    request: BulkArchiveRequest,
  ): Promise<BulkOperationResponse> {
    return this._post("/api/v1/conversations/bulk/archive", request);
  }

  /** Bulk move conversations to folder. */
  async bulkMove(
    request: BulkMoveRequest,
  ): Promise<BulkOperationResponse> {
    return this._post("/api/v1/conversations/bulk/move", request);
  }
}
