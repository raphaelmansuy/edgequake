/**
 * @module chat-api
 * @description Unified Chat API Client
 *
 * This module provides the client for the unified chat completions API.
 * The server-side endpoint handles message persistence, so the client
 * only needs to focus on displaying the response.
 *
 * @implements FEAT0772 - Chat completions API client
 * @implements FEAT0773 - Streaming chat responses
 * @implements FEAT0774 - Query mode selection
 *
 * @enforces BR0703 - Auto-create conversation if none provided
 * @enforces BR0704 - Stream events include chunk, sources, done
 *
 * Key benefits:
 * - No client-side message saving required
 * - Automatic conversation creation/selection
 * - Single API call for complete chat flow
 */

import { buildQueryContextFromRetrieval } from "@/lib/utils/source-mapper";
import type { SubgraphBundle } from "@/lib/utils/subgraph-types";
import type { DocumentFilter, QueryContext, QueryMode } from "@/types";
import { apiClient, streamClient } from "./client";

// ============================================================================
// Types
// ============================================================================

/**
 * Chat completion request sent to the server.
 */
export interface ChatCompletionRequest {
  /** Existing conversation ID. If null, creates a new conversation. */
  conversation_id?: string;
  /** User message content. */
  message: string;
  /** Query mode (naive, local, global, hybrid, mix, bypass). */
  mode?: QueryMode;
  /** Whether to stream the response. */
  stream?: boolean;
  /** Maximum tokens for response. */
  max_tokens?: number;
  /** Temperature for generation (0.0-2.0). */
  temperature?: number;
  /** Top K for retrieval. */
  top_k?: number;
  /** Parent message ID for threading. */
  parent_id?: string;
  /**
   * LLM provider ID to use for this query (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Provider selection in query interface
   */
  provider?: string;
  /**
   * Specific model name within the provider (e.g., "gpt-4o-mini", "gemma3:12b").
   * When combined with provider, allows full model selection from models.toml.
   * @implements SPEC-032: Full model selection in query interface
   */
  model?: string;
  /**
   * SPEC-109: reasoning effort override (`none`/`minimal`/`low`/…). Omit = Auto.
   */
  reasoning_effort?: string;
  /**
   * Preferred response language (ISO 639-1 code, e.g., "en", "zh", "fr").
   * When provided, the backend instructs the LLM to respond in this language
   * regardless of the query language. Falls back to query language detection
   * when not set.
   */
  language?: string;
  /**
   * Optional system prompt extension. Injected between the base RAG instructions
   * and the retrieved context to steer LLM behaviour without replacing the core prompt.
   * @implements SPEC-004: System prompt extension point
   */
  system_prompt?: string;
  /**
   * Optional document filter to restrict RAG context to matching documents.
   * @implements SPEC-005: Document filters for queries
   */
  document_filter?: DocumentFilter;
  /**
   * Optional image attachments for vision-capable models.
   * Each image is base64-encoded with its MIME type.
   * Max 4 images, each ≤ 20 MiB. Accepted: image/jpeg, image/png, image/gif, image/webp.
   *
   * @implements Issue #203: Image upload support
   */
  images?: Array<{ data: string; mime_type: string }>;
  /**
   * SPEC-180: one image to search by. Sent as the multipart part `query_image`; the rest of
   * the request rides in the part `request`. Only a workspace that embeds images accepts it.
   */
  query_image?: File;
  /**
   * Payload tier for source snippets: citation (200 chars) | agent (full chunk).
   * @implements SPEC-037 + SPEC-028
   */
  content_granularity?: 'citation' | 'agent' | 'debug';
  /**
   * Graph entity ids to admit before ANN (Ask companion seed).
   * Does not change query mode.
   */
  seed_entity_ids?: string[];
}

/**
 * Source reference returned with query results.
 * @implements SPEC-006: Enriched source references
 */
export interface SourceReference {
  source_type: string;
  id: string;
  score: number;
  rerank_score?: number;
  snippet?: string;
  reference_id?: number;
  document_id?: string;
  file_path?: string;
  /** Source line range start (for line-level citation). */
  start_line?: number;
  /** Source line range end (for line-level citation). */
  end_line?: number;
  /** Chunk position index within the document. */
  chunk_index?: number;
  /**
   * PDF page number (1-indexed) where this chunk starts.
   * Present only for PDFs processed with page-aware chunking (SPEC-033).
   * Enables deep links to the exact PDF page.
   */
  page_start?: number;
  /** PDF page number (1-indexed) where this chunk ends. Always equals page_start. */
  page_end?: number;
  /** Entity type (e.g., "PERSON", "ORGANIZATION"). @implements SPEC-006 */
  entity_type?: string;
  /** Entity degree (number of relationships). @implements SPEC-006 */
  degree?: number;
  /** Source chunk IDs that mention this entity. @implements SPEC-006 */
  source_chunk_ids?: string[];
}

/**
 * Query statistics.
 */
export interface QueryStats {
  embedding_time_ms: number;
  retrieval_time_ms: number;
  generation_time_ms: number;
  total_time_ms: number;
  sources_retrieved: number;
  rerank_time_ms?: number;
}

/**
 * Non-streaming chat completion response.
 */
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
  /** LLM provider used (lineage tracking). @implements SPEC-032 */
  llm_provider?: string;
  /** LLM model used (lineage tracking). @implements SPEC-032 */
  llm_model?: string;
}

/**
 * Streaming SSE event types.
 */
export type ChatStreamEvent =
  | {
      type: "conversation";
      conversation_id: string;
      user_message_id: string;
    }
  | {
      type: "context";
      sources: SourceReference[];
      /** Structured query-matched graph (SPEC-028). */
      subgraph?: SubgraphBundle;
      /** Query mode used for retrieval. @implements SPEC-006 */
      query_mode?: string;
      /** Retrieval time in milliseconds. @implements SPEC-006 */
      retrieval_time_ms?: number;
    }
  | {
      type: "token";
      content: string;
    }
  | {
      type: "thinking";
      content: string;
    }
  | {
      type: "stage";
      stage: "retrieving" | "reading" | "generating";
      detail?: string;
    }
  | {
      type: "done";
      assistant_message_id: string;
      tokens_used: number;
      duration_ms: number;
      /** LLM provider used (lineage tracking). @implements SPEC-032 */
      llm_provider?: string;
      /** LLM model used (lineage tracking). @implements SPEC-032 */
      llm_model?: string;
      /** SPEC-142: verified answer with document+page links. */
      answer?: string;
    }
  | {
      /** Auto-generated conversation title. @implements FEAT0505 */
      type: "title_update";
      conversation_id: string;
      title: string;
    }
  | {
      type: "error";
      message: string;
      code: string;
    };

// ============================================================================
// API Functions
// ============================================================================

/**
 * One body builder for both chat calls. JSON, or multipart when a query image is attached
 * (the browser sets the boundary, so no Content-Type is written here).
 */
export function chatRequestBody(
  request: ChatCompletionRequest,
  stream: boolean,
): string | FormData {
  const { query_image: queryImage, ...rest } = request;
  const json = JSON.stringify({ ...rest, stream });
  if (!queryImage) return json;
  const form = new FormData();
  form.append("request", json);
  form.append("query_image", queryImage, queryImage.name);
  return form;
}

/**
 * Send a chat completion request (non-streaming).
 *
 * @param request - Chat completion request
 * @returns Chat completion response with all IDs and content
 */
export async function chatCompletion(
  request: ChatCompletionRequest,
): Promise<ChatCompletionResponse> {
  return apiClient<ChatCompletionResponse>("/chat/completions", {
    method: "POST",
    body: chatRequestBody(request, false),
  });
}

/**
 * Send a streaming chat completion request.
 *
 * Usage:
 * ```typescript
 * for await (const event of chatCompletionStream({ message: "Hello" })) {
 *   switch (event.type) {
 *     case "conversation":
 *       console.log("Conversation ID:", event.conversation_id);
 *       break;
 *     case "token":
 *       console.log("Token:", event.content);
 *       break;
 *     case "done":
 *       console.log("Completed:", event.assistant_message_id);
 *       break;
 *     case "error":
 *       console.error("Error:", event.message);
 *       break;
 *   }
 * }
 * ```
 *
 * @param request - Chat completion request
 * @yields ChatStreamEvent objects as they arrive
 */
export async function* chatCompletionStream(
  request: ChatCompletionRequest,
): AsyncGenerator<ChatStreamEvent, void, unknown> {
  yield* streamClient<ChatStreamEvent>("/chat/completions/stream", {
    method: "POST",
    body: chatRequestBody(request, true),
  });
}

// ============================================================================
// Helper Types for UI Integration
// ============================================================================

/**
 * Accumulated state during streaming.
 * Useful for building UI components.
 */
export interface StreamingState {
  /** Current streaming status */
  status: "idle" | "streaming" | "done" | "error";
  /** Accumulated content so far */
  content: string;
  /** Conversation ID (available after first event) */
  conversationId?: string;
  /** User message ID (available after first event) */
  userMessageId?: string;
  /** Assistant message ID (available after done event) */
  assistantMessageId?: string;
  /** Context sources (if received) */
  sources?: SourceReference[];
  /** Structured subgraph from context event (SPEC-028). */
  subgraph?: SubgraphBundle;
  /** UI-ready context built from sources + subgraph. */
  queryContext?: QueryContext;
  /** Query mode used for retrieval. @implements SPEC-006 */
  queryMode?: string;
  /** Retrieval time in ms. @implements SPEC-006 */
  retrievalTimeMs?: number;
  /** Token count (available after done event) */
  tokensUsed?: number;
  /** Duration in ms (available after done event) */
  durationMs?: number;
  /** LLM provider used (lineage). @implements SPEC-032 */
  llmProvider?: string;
  /** LLM model used (lineage). @implements SPEC-032 */
  llmModel?: string;
  /** Error message if status is "error" */
  error?: string;
}

/**
 * Process streaming events and accumulate state.
 * Useful for React components with useState.
 *
 * @param event - The streaming event
 * @param currentState - Current accumulated state
 * @returns Updated state
 */
export function reduceStreamingEvent(
  event: ChatStreamEvent,
  currentState: StreamingState,
): StreamingState {
  switch (event.type) {
    case "conversation":
      return {
        ...currentState,
        status: "streaming",
        conversationId: event.conversation_id,
        userMessageId: event.user_message_id,
      };
    case "context":
      return {
        ...currentState,
        sources: event.sources,
        subgraph: event.subgraph,
        queryContext: buildQueryContextFromRetrieval(
          event.sources,
          event.subgraph,
        ),
        queryMode: event.query_mode,
        retrievalTimeMs: event.retrieval_time_ms,
      };
    case "token":
      return {
        ...currentState,
        content: currentState.content + event.content,
      };
    case "thinking":
      // Could store thinking content separately if needed
      return currentState;
    case "stage":
      return currentState;
    case "done":
      return {
        ...currentState,
        status: "done",
        // SPEC-142: prefer verified answer when present.
        content: event.answer ?? currentState.content,
        assistantMessageId: event.assistant_message_id,
        tokensUsed: event.tokens_used,
        durationMs: event.duration_ms,
        llmProvider: event.llm_provider,
        llmModel: event.llm_model,
      };
    case "error":
      return {
        ...currentState,
        status: "error",
        error: event.message,
      };
    case "title_update":
      // Title updates don't affect streaming state
      return currentState;
    default:
      return currentState;
  }
}

/**
 * Create initial streaming state.
 */
export function createInitialStreamingState(): StreamingState {
  return {
    status: "idle",
    content: "",
  };
}
