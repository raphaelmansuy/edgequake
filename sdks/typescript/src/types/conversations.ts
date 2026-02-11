/**
 * Conversation and message types.
 *
 * @module types/conversations
 * @see edgequake/crates/edgequake-api/src/handlers/conversations_types.rs
 */

import type { Timestamp } from "./common.js";

// ── Conversations ─────────────────────────────────────────────

export interface ListConversationsQuery {
  limit?: number;
  offset?: number;
  folder_id?: string;
  search?: string;
  archived?: boolean;
}

export interface ConversationInfo {
  id: string;
  title: string;
  created_at: Timestamp;
  updated_at: Timestamp;
  message_count: number;
  folder_id?: string;
  is_archived: boolean;
  is_shared: boolean;
}

export interface ConversationDetail extends ConversationInfo {
  messages: MessageInfo[];
  metadata?: Record<string, unknown>;
}

export interface CreateConversationRequest {
  title?: string;
  folder_id?: string;
  metadata?: Record<string, unknown>;
}

export interface UpdateConversationRequest {
  title?: string;
  folder_id?: string;
  is_archived?: boolean;
}

export interface ConversationResponse {
  id: string;
  title: string;
  created_at: Timestamp;
  updated_at: Timestamp;
}

export interface ImportConversationsRequest {
  conversations: Array<{
    title: string;
    messages: Array<{ role: string; content: string }>;
  }>;
}

export interface ImportResponse {
  imported: number;
  failed: number;
  conversation_ids: string[];
}

/** Alias for ImportResponse. */
export type ImportConversationsResponse = ImportResponse;

export interface ShareResponse {
  share_id: string;
  share_url: string;
}

// ── Messages ──────────────────────────────────────────────────

export interface MessageInfo {
  id: string;
  conversation_id: string;
  role: string;
  content: string;
  created_at: Timestamp;
  metadata?: Record<string, unknown>;
}

export interface CreateMessageRequest {
  role: string;
  content: string;
  metadata?: Record<string, unknown>;
}

export interface UpdateMessageRequest {
  content?: string;
  metadata?: Record<string, unknown>;
}

export interface MessageResponse extends MessageInfo {}

// ── Folders ───────────────────────────────────────────────────

export interface FolderInfo {
  id: string;
  name: string;
  conversation_count: number;
  created_at: Timestamp;
}

export interface CreateFolderRequest {
  name: string;
}

export interface UpdateFolderRequest {
  name?: string;
}

// ── Shared ────────────────────────────────────────────────────

export interface SharedConversation {
  share_id: string;
  conversation: ConversationDetail;
}

// ── Bulk Operations ──────────────────────────────────────────

export interface BulkDeleteRequest {
  conversation_ids: string[];
}

export interface BulkArchiveRequest {
  conversation_ids: string[];
  archive: boolean;
}

export interface BulkMoveRequest {
  conversation_ids: string[];
  folder_id: string;
}
