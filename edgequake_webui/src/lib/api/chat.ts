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
  /** Query mode (local, global, hybrid, naive). */
  mode?: "local" | "global" | "hybrid" | "naive";
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
}

/**
 * Source reference returned with query results.
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
      type: "done";
      assistant_message_id: string;
      tokens_used: number;
      duration_ms: number;
      /** LLM provider used (lineage tracking). @implements SPEC-032 */
      llm_provider?: string;
      /** LLM model used (lineage tracking). @implements SPEC-032 */
      llm_model?: string;
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
 * Send a chat completion request (non-streaming).
 *
 * @param request - Chat completion request
 * @returns Chat completion response with all IDs and content
 */
export async function chatCompletion(
  request: ChatCompletionRequest
): Promise<ChatCompletionResponse> {
  return apiClient<ChatCompletionResponse>("/chat/completions", {
    method: "POST",
    body: JSON.stringify({ ...request, stream: false }),
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
  request: ChatCompletionRequest
): AsyncGenerator<ChatStreamEvent, void, unknown> {
  yield* streamClient<ChatStreamEvent>("/chat/completions/stream", {
    method: "POST",
    body: JSON.stringify({ ...request, stream: true }),
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
  /** Token count (available after done event) */
  tokensUsed?: number;
  /** Duration in ms (available after done event) */
  durationMs?: number;
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
  currentState: StreamingState
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
      };
    case "token":
      return {
        ...currentState,
        content: currentState.content + event.content,
      };
    case "thinking":
      // Could store thinking content separately if needed
      return currentState;
    case "done":
      return {
        ...currentState,
        status: "done",
        assistantMessageId: event.assistant_message_id,
        tokensUsed: event.tokens_used,
        durationMs: event.duration_ms,
      };
    case "error":
      return {
        ...currentState,
        status: "error",
        error: event.message,
      };
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
