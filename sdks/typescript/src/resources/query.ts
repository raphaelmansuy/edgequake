/**
 * Query resource — execute RAG queries.
 *
 * @module resources/query
 * @see edgequake/crates/edgequake-api/src/handlers/query.rs
 */

import type {
  QueryRequest,
  QueryResponse,
  QueryStreamEvent,
  StreamQueryRequest,
} from "../types/query.js";
import { Resource } from "./base.js";

export class QueryResource extends Resource {
  /** Execute a RAG query and get a complete response. */
  async execute(request: QueryRequest): Promise<QueryResponse> {
    return this._post("/api/v1/query", request);
  }

  /**
   * Execute a streaming RAG query.
   * Returns an async iterator of query stream events.
   */
  stream(
    request: StreamQueryRequest,
    signal?: AbortSignal,
  ): AsyncIterable<QueryStreamEvent> {
    return this._streamSSE<QueryStreamEvent>(
      "/api/v1/query/stream",
      request,
      signal,
    );
  }
}
