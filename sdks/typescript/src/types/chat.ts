/**
 * Chat types.
 *
 * @module types/chat
 * @see edgequake/crates/edgequake-api/src/handlers/chat_types.rs
 */

import type { ConversationMessage } from "./query.js";

// ── Request ───────────────────────────────────────────────────

export interface ChatCompletionRequest {
  message: string;
  conversation_id?: string;
  mode?: "naive" | "local" | "global" | "hybrid" | "mix";
  stream?: boolean;
  conversation_history?: ConversationMessage[];
  enable_rerank?: boolean;
  llm_provider?: string;
  llm_model?: string;
}

// ── Response ──────────────────────────────────────────────────

/** Source reference in chat responses. */
export interface SourceReference {
  content: string;
  document_id?: string;
  chunk_id?: string;
  score?: number;
}

/** Statistics for query generation. */
export interface QueryStats {
  entities_found?: number;
  relationships_found?: number;
  chunks_retrieved?: number;
}

/** Non-streaming chat completion response matching Rust ChatCompletionResponse. */
export interface ChatCompletionResponse {
  /** Conversation ID (created or existing). */
  conversation_id: string;
  /** User message ID. */
  user_message_id: string;
  /** Assistant message ID. */
  assistant_message_id: string;
  /** Assistant response content. */
  content: string;
  /** Query mode used. */
  mode: string;
  /** Sources retrieved. */
  sources: SourceReference[];
  /** Generation statistics. */
  stats: QueryStats;
  /** Tokens used for generation. */
  tokens_used: number;
  /** Duration in milliseconds. */
  duration_ms: number;
  /** LLM provider used (lineage tracking). */
  llm_provider?: string;
  /** LLM model used (lineage tracking). */
  llm_model?: string;
}

// ── Stream Events ─────────────────────────────────────────────

/** Chat streaming SSE events matching Rust ChatStreamEvent enum. */
export type ChatStreamEvent =
  | { type: "conversation"; conversation_id: string; user_message_id: string }
  | { type: "context"; sources: SourceReference[] }
  | { type: "token"; content: string }
  | { type: "thinking"; content: string }
  | {
      type: "done";
      assistant_message_id: string;
      tokens_used: number;
      duration_ms: number;
      llm_provider?: string;
      llm_model?: string;
    }
  | { type: "error"; message: string; code: string };
