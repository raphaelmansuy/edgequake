/**
 * Chat types.
 *
 * @module types/chat
 * @see edgequake/crates/edgequake-api/src/handlers/chat_types.rs
 */

import type { QueryStats, SourceReference } from "./query.js";

// Re-export for consumers that import from chat module
export type { QueryStats, SourceReference };

// ── Request ───────────────────────────────────────────────────

/** Unified chat completion request matching Rust ChatCompletionRequest. */
export interface ChatCompletionRequest {
  /** User message content. */
  message: string;
  /** Existing conversation ID. If null, creates a new conversation. */
  conversation_id?: string;
  /** Query mode (local, global, hybrid, naive). */
  mode?: "naive" | "local" | "global" | "hybrid" | "mix";
  /** Whether to stream the response (defaults to true). */
  stream?: boolean;
  /** Maximum tokens for response. */
  max_tokens?: number;
  /** Temperature for generation (0.0-2.0). */
  temperature?: number;
  /** Top K for retrieval. */
  top_k?: number;
  /** Parent message ID for threading. */
  parent_id?: string;
  /** LLM provider ID (e.g., "openai", "ollama", "lmstudio"). */
  provider?: string;
  /** Specific model name within the provider (e.g., "gpt-4o-mini", "gemma3:12b"). */
  model?: string;
}

// ── Response ──────────────────────────────────────────────────

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
  | { type: "error"; message: string; code: string }
  | {
      /** Auto-generated conversation title. @implements FEAT0505 */
      type: "title_update";
      conversation_id: string;
      title: string;
    };
