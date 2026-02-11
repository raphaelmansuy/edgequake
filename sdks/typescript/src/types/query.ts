/**
 * Query types.
 *
 * @module types/query
 * @see edgequake/crates/edgequake-api/src/handlers/query_types.rs
 */

// ── Request ───────────────────────────────────────────────────

export interface ConversationMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface QueryRequest {
  query: string;
  mode?: "naive" | "local" | "global" | "hybrid" | "mix";
  context_only?: boolean;
  prompt_only?: boolean;
  include_references?: boolean;
  max_results?: number;
  conversation_history?: ConversationMessage[];
  enable_rerank?: boolean;
  rerank_model?: string;
  rerank_top_k?: number;
  llm_provider?: string;
  llm_model?: string;
}

export interface StreamQueryRequest {
  query: string;
  mode?: "naive" | "local" | "global" | "hybrid" | "mix";
}

// ── Response ──────────────────────────────────────────────────

export interface QuerySource {
  content: string;
  document_id?: string;
  file_path?: string;
  reference_id?: string;
  score?: number;
}

export interface QueryResponse {
  answer: string;
  mode: string;
  sources: QuerySource[];
  context?: string;
  prompt?: string;
  tokens_used?: number;
}

// ── Stream Events ─────────────────────────────────────────────

export interface QueryStreamEvent {
  chunk: string;
}
