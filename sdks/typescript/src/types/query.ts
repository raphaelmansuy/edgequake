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
  /** LLM provider to use for this query (e.g., "openai", "ollama", "lmstudio"). */
  llm_provider?: string;
  /** Specific model name within the provider (e.g., "gpt-4o-mini", "gemma3:12b"). */
  llm_model?: string;
}

export interface StreamQueryRequest {
  query: string;
  mode?: "naive" | "local" | "global" | "hybrid" | "mix";
}

// ── Shared Response Types ─────────────────────────────────────

/** A source reference in query/chat responses. Matches Rust SourceReference. */
export interface SourceReference {
  /** Source type (chunk, entity, relationship). */
  source_type: string;
  /** Source ID. */
  id: string;
  /** Relevance score. */
  score: number;
  /** Rerank score (if reranking was applied). */
  rerank_score?: number;
  /** Content snippet. */
  snippet?: string;
  /** Reference ID for citation (1, 2, 3, ...). */
  reference_id?: number;
  /** Document ID that this reference came from. */
  document_id?: string;
  /** Original file path of the source document. */
  file_path?: string;
  /** Start line number in the document. */
  start_line?: number;
  /** End line number in the document. */
  end_line?: number;
  /** Chunk index in the document. */
  chunk_index?: number;
}

/** Query statistics. Matches Rust QueryStats. */
export interface QueryStats {
  /** Embedding time in ms. */
  embedding_time_ms: number;
  /** Retrieval time in ms. */
  retrieval_time_ms: number;
  /** Generation time in ms. */
  generation_time_ms: number;
  /** Total time in ms. */
  total_time_ms: number;
  /** Number of sources retrieved. */
  sources_retrieved: number;
  /** Rerank time in ms (if reranking was applied). */
  rerank_time_ms?: number;
  /** Number of tokens generated. */
  tokens_used?: number;
  /** Tokens per second generation speed. */
  tokens_per_second?: number;
  /** LLM provider used for generation. */
  llm_provider?: string;
  /** LLM model name used for generation. */
  llm_model?: string;
}

// ── Response ──────────────────────────────────────────────────

export interface QueryResponse {
  /** Generated answer. */
  answer: string;
  /** Query mode used. */
  mode: string;
  /** Retrieved context sources. */
  sources: SourceReference[];
  /** Query statistics. */
  stats: QueryStats;
  /** Conversation ID for multi-turn context. */
  conversation_id?: string;
  /** Whether reranking was applied. */
  reranked?: boolean;
}

/** @deprecated Use {@link SourceReference} instead. */
export type QuerySource = SourceReference;

// ── Stream Events ─────────────────────────────────────────────

export interface QueryStreamEvent {
  chunk: string;
}
