/** Server-synced conversation and folder types. */

import type { QueryMode } from "./query";

export type ConversationMode = QueryMode;

export interface ServerConversation {
  id: string;
  tenant_id: string;
  workspace_id?: string | null;
  user_id: string;
  title: string;
  mode: ConversationMode;
  is_pinned: boolean;
  is_archived: boolean;
  folder_id?: string | null;
  share_id?: string | null;
  message_count: number;
  last_message_preview?: string | null;
  meta: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface ConversationWithMessages extends ServerConversation {
  messages: ServerMessage[];
}

export interface ServerMessage {
  id: string;
  conversation_id: string;
  parent_id?: string | null;
  role: "user" | "assistant" | "system";
  content: string;
  mode?: ConversationMode | null;
  tokens_used?: number | null;
  duration_ms?: number | null;
  thinking_time_ms?: number | null;
  context?: ServerMessageContext | null;
  is_error: boolean;
  created_at: string;
  updated_at: string;
  /** LLM provider used (lineage tracking). @implements SPEC-032 */
  llm_provider?: string | null;
  /** LLM model used (lineage tracking). @implements SPEC-032 */
  llm_model?: string | null;
}

export interface ServerMessageContext {
  sources?: MessageSource[];
  entities?: ServerContextEntity[];
  relationships?: ServerContextRelationship[];
  thinking?: string;
}

export interface MessageSource {
  id: string;
  title?: string;
  content: string;
  score: number;
  /** Source type: chunk, entity, or relationship */
  source_type?: string;
  /** Document ID for citation link */
  document_id?: string;
  /** Original file path for citation display */
  file_path?: string;
}

/** Entity returned in context with source tracking */
export interface ServerContextEntity {
  name: string;
  entity_type: string;
  description?: string;
  score: number;
  /** Source document ID for citation link */
  source_document_id?: string;
  /** Original file path for citation display */
  source_file_path?: string;
  /** Source chunk IDs for provenance */
  source_chunk_ids?: string[];
}

/** Relationship returned in context with source tracking */
export interface ServerContextRelationship {
  source: string;
  target: string;
  relation_type: string;
  description?: string;
  score: number;
  /** Source document ID for citation link */
  source_document_id?: string;
  /** Original file path for citation display */
  source_file_path?: string;
}

export interface ConversationFolder {
  id: string;
  tenant_id: string;
  workspace_id?: string | null;
  user_id: string;
  name: string;
  parent_id?: string | null;
  position: number;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Conversation Request/Response Types
// ============================================================================

export interface CreateConversationRequest {
  title?: string;
  mode?: ConversationMode;
  folder_id?: string | null;
}

export interface UpdateConversationRequest {
  title?: string;
  mode?: ConversationMode;
  is_pinned?: boolean;
  is_archived?: boolean;
  folder_id?: string | null;
}

export interface CreateMessageRequest {
  content: string;
  role: "user";
  parent_id?: string | null;
  stream?: boolean;
}

export interface UpdateMessageRequest {
  content?: string;
  tokens_used?: number;
  duration_ms?: number;
  thinking_time_ms?: number;
  context?: ServerMessageContext;
  is_error?: boolean;
}

// ============================================================================
// Conversation Pagination Types
// ============================================================================

export interface CursorPaginationParams {
  cursor?: string;
  limit?: number;
}

export interface ConversationFilterParams {
  mode?: ConversationMode[];
  archived?: boolean;
  pinned?: boolean;
  folder_id?: string;
  /** When true, returns only conversations without any folder (unfiled). */
  unfiled?: boolean;
  search?: string;
  date_from?: string;
  date_to?: string;
  sort?: "updated_at" | "created_at" | "title";
  order?: "asc" | "desc";
}

export interface PaginatedConversations {
  items: ServerConversation[];
  pagination: CursorPaginationMeta;
}

export interface PaginatedMessages {
  items: ServerMessage[];
  pagination: CursorPaginationMeta;
}

export interface CursorPaginationMeta {
  next_cursor?: string | null;
  prev_cursor?: string | null;
  total: number;
  has_more: boolean;
}

export interface ShareConversationResponse {
  share_id: string;
  share_url: string;
}

// ============================================================================
// Conversation Import Types (localStorage migration)
// ============================================================================

export interface ImportConversationsRequest {
  conversations: LocalStorageConversation[];
}

export interface LocalStorageConversation {
  id: string;
  title: string;
  messages: {
    id: string;
    role: "user" | "assistant";
    content: string;
    mode?: ConversationMode;
    tokensUsed?: number;
    durationMs?: number;
    thinkingTimeMs?: number;
    context?: ServerMessageContext;
    isError?: boolean;
    timestamp?: number;
  }[];
  createdAt: number;
  updatedAt: number;
}

export interface ImportConversationsResponse {
  imported: number;
  failed: number;
  errors?: { id: string; error: string }[];
}

// ============================================================================
// Folder Request Types
// ============================================================================

export interface CreateFolderRequest {
  name: string;
  parent_id?: string | null;
}

export interface UpdateFolderRequest {
  name?: string;
  parent_id?: string | null;
  position?: number;
}
