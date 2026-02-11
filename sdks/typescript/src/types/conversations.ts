/**
 * Conversation, message, folder, and sharing types.
 *
 * WHY: Types match Rust DTOs in conversations_types.rs exactly.
 * @module types/conversations
 * @see edgequake/crates/edgequake-api/src/handlers/conversations_types.rs
 */

import type { Timestamp } from "./common.js";

// ── Conversations ─────────────────────────────────────────────

/**
 * Query parameters for listing conversations.
 * WHY: Rust uses cursor-based pagination with filter[] bracket params.
 */
export interface ListConversationsQuery {
  /** Cursor for pagination (cursor-based, not offset). */
  cursor?: string;
  /** Max items to return (default 20, max 100). */
  limit?: number;
  /** Filter by mode (comma-separated: local,global,hybrid). */
  filter_mode?: string;
  /** Filter by archived status. */
  filter_archived?: boolean;
  /** Filter by pinned status. */
  filter_pinned?: boolean;
  /** Filter by folder ID. */
  filter_folder_id?: string;
  /** Search in title. */
  filter_search?: string;
  /** Sort field (updated_at, created_at, title). */
  sort?: string;
  /** Sort order (asc, desc). */
  order?: string;
}

/**
 * Conversation response DTO.
 * WHY: Matches Rust ConversationResponse exactly.
 */
export interface ConversationInfo {
  id: string;
  tenant_id: string;
  workspace_id?: string;
  title: string;
  mode: string;
  is_pinned: boolean;
  is_archived: boolean;
  folder_id?: string;
  share_id?: string;
  message_count?: number;
  last_message_preview?: string;
  created_at: Timestamp;
  updated_at: Timestamp;
}

/**
 * Conversation with messages (from GET /conversations/:id).
 * WHY: Rust returns ConversationWithMessagesResponse = { conversation, messages }.
 */
export interface ConversationDetail {
  conversation: ConversationInfo;
  messages: MessageInfo[];
}

/** WHY: Matches Rust CreateConversationApiRequest. */
export interface CreateConversationRequest {
  title?: string;
  mode?: string;
  folder_id?: string;
}

/** WHY: Matches Rust UpdateConversationApiRequest. */
export interface UpdateConversationRequest {
  title?: string;
  mode?: string;
  is_pinned?: boolean;
  is_archived?: boolean;
  folder_id?: string;
}

/** Paginated conversations list response. */
export interface PaginatedConversationsResponse {
  items: ConversationInfo[];
  pagination: PaginationMeta;
}

/** Cursor-based pagination metadata. */
export interface PaginationMeta {
  next_cursor?: string;
  prev_cursor?: string;
  total?: number;
  has_more: boolean;
}

/** WHY: Rust accepts Vec<serde_json::Value> — raw JSON objects. */
export interface ImportConversationsRequest {
  conversations: unknown[];
}

/** WHY: Matches Rust ImportConversationsResponse. */
export interface ImportConversationsResponse {
  imported: number;
  failed: number;
  errors: ImportError[];
}

export interface ImportError {
  id: string;
  error: string;
}

export interface ShareResponse {
  share_id: string;
  share_url: string;
}

// ── Messages ──────────────────────────────────────────────────

/**
 * Message response DTO.
 * WHY: Matches Rust MessageResponse exactly.
 */
export interface MessageInfo {
  id: string;
  conversation_id: string;
  parent_id?: string;
  role: string;
  content: string;
  mode?: string;
  tokens_used?: number;
  duration_ms?: number;
  thinking_time_ms?: number;
  context?: unknown;
  is_error: boolean;
  created_at: Timestamp;
  updated_at: Timestamp;
}

/** Paginated messages list response. */
export interface PaginatedMessagesResponse {
  items: MessageInfo[];
  pagination: PaginationMeta;
}

/** Query params for listing messages. */
export interface ListMessagesQuery {
  cursor?: string;
  limit?: number;
}

/** WHY: Matches Rust CreateMessageApiRequest. */
export interface CreateMessageRequest {
  content: string;
  role: string;
  parent_id?: string;
  /** Whether to stream response (default true in Rust). */
  stream?: boolean;
}

/** WHY: Matches Rust UpdateMessageApiRequest. */
export interface UpdateMessageRequest {
  content?: string;
  tokens_used?: number;
  duration_ms?: number;
  thinking_time_ms?: number;
  context?: unknown;
  is_error?: boolean;
}

// ── Folders ───────────────────────────────────────────────────

/**
 * Folder response DTO.
 * WHY: Matches Rust FolderResponse exactly.
 */
export interface FolderInfo {
  id: string;
  tenant_id: string;
  workspace_id?: string;
  name: string;
  parent_id?: string;
  position: number;
  created_at: Timestamp;
  updated_at: Timestamp;
}

/** WHY: Matches Rust CreateFolderApiRequest. */
export interface CreateFolderRequest {
  name: string;
  parent_id?: string;
}

/** WHY: Matches Rust UpdateFolderApiRequest. */
export interface UpdateFolderRequest {
  name?: string;
  parent_id?: string;
  position?: number;
}

// ── Shared ────────────────────────────────────────────────────

/** Shared conversation accessed via public share link. */
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
  /** Target folder ID (null/undefined = move to root). */
  folder_id?: string;
}
