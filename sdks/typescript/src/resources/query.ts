/**
 * Query resource — execute RAG queries.
 *
 * @module resources/query
 * @see edgequake/crates/edgequake-api/src/handlers/query.rs
 */

import type { RequestOptions } from "../transport/types.js";
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
   *
   * WHY: The query stream endpoint returns raw text SSE chunks (not JSON),
   * so we cannot use _streamSSE which tries JSON.parse. Instead, we wrap
   * each raw text chunk as { chunk: data } to match QueryStreamEvent.
   */
  stream(
    request: StreamQueryRequest,
    signal?: AbortSignal,
  ): AsyncIterable<QueryStreamEvent> {
    const self = this;
    return {
      [Symbol.asyncIterator]() {
        return (async function* () {
          const options: RequestOptions = {
            method: "POST",
            path: "/api/v1/query/stream",
            body: request,
            signal,
          };
          for await (const data of self.transport.stream(options)) {
            // WHY: Stream data may be raw text OR JSON. Try JSON first.
            try {
              const parsed = JSON.parse(data) as QueryStreamEvent;
              yield parsed;
            } catch {
              // Raw text chunk — wrap as QueryStreamEvent
              yield { chunk: data };
            }
          }
        })();
      },
    };
  }
}
