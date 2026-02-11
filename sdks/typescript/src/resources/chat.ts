/**
 * Chat resource — chat completions (unified chat API).
 *
 * @module resources/chat
 * @see edgequake/crates/edgequake-api/src/handlers/chat.rs
 */

import type {
  ChatCompletionRequest,
  ChatCompletionResponse,
  ChatStreamEvent,
} from "../types/chat.js";
import { Resource } from "./base.js";

export class ChatResource extends Resource {
  /** Send a chat completion request and get a complete response. */
  async completions(
    request: ChatCompletionRequest,
  ): Promise<ChatCompletionResponse> {
    return this._post("/api/v1/chat/completions", request);
  }

  /**
   * Send a streaming chat completion request.
   * Returns an async iterator of chat stream events.
   */
  stream(
    request: ChatCompletionRequest,
    signal?: AbortSignal,
  ): AsyncIterable<ChatStreamEvent> {
    return this._streamSSE<ChatStreamEvent>(
      "/api/v1/chat/completions/stream",
      request,
      signal,
    );
  }
}
