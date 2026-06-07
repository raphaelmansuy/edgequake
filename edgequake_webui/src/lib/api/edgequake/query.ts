/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { api, streamClient } from "../client";

import type {
  QueryRequest,
  QueryResponse,
  QueryStreamChunk,
} from "@/types";

export async function query(request: QueryRequest): Promise<QueryResponse> {
  return api.post<QueryResponse>("/query", request);
}

export async function* queryStream(
  request: QueryRequest,
): AsyncGenerator<QueryStreamChunk, void, unknown> {
  yield* streamClient<QueryStreamChunk>("/query/stream", {
    method: "POST",
    body: JSON.stringify({ ...request, stream: true }),
  });
}
