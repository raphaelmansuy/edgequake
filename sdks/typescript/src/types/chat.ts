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

export interface ChatCompletionResponse {
  message: string;
  conversation_id: string;
  sources?: Array<{
    content: string;
    document_id?: string;
    score?: number;
  }>;
  tokens_used?: number;
}

// ── Stream Events ─────────────────────────────────────────────

export type ChatStreamEvent =
  | { type: "content"; delta: string }
  | { type: "sources"; sources: Array<{ content: string; score?: number }> }
  | { type: "done"; conversation_id: string }
  | { type: "error"; message: string };
