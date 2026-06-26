/**
 * @module api-stream-client
 * @description SSE (Server-Sent Events) streaming fetch client.
 *
 * Extracted from `client.ts` (SPEC-017 LOC guard + SRP): this module owns only
 * the streaming transport — reading a fetch `Response.body` ReadableStream,
 * splitting SSE frames, and yielding parsed JSON or raw token events.
 *
 * @implements FEAT0770 - SSE streaming client
 */
import { getRuntimeApiBaseUrl } from "@/lib/runtime-config";
import { buildHeaders, handleErrorResponse } from "./client";

/**
 * Streaming API client for SSE (Server-Sent Events) responses.
 *
 * SSE wire format: `"data: <content>\n\n"` per event. Multi-line payloads use
 * repeated `data:` lines that must be concatenated without separators.
 */
export async function* streamClient<T>(
  endpoint: string,
  options: RequestInit = {},
): AsyncGenerator<T, void, unknown> {
  const url = endpoint.startsWith("http")
    ? endpoint
    : `${getRuntimeApiBaseUrl()}${endpoint}`;

  const config: RequestInit = {
    ...options,
    headers: buildHeaders(options.headers),
  };

  const response = await fetch(url, config);

  if (!response.ok) {
    throw await handleErrorResponse(response);
  }

  if (!response.body) {
    throw new Error("Response body is null");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      const { done, value } = await reader.read();

      if (done) {
        if (buffer.trim()) {
          const parsed = parseSSEData(buffer);
          if (parsed !== null) {
            yield parsed as T;
          }
        }
        break;
      }

      buffer += decoder.decode(value, { stream: true });

      // Process complete SSE events (separated by double newlines)
      const events = buffer.split("\n\n");
      buffer = events.pop() || "";

      for (const event of events) {
        const parsed = parseSSEData(event);
        if (parsed !== null) {
          yield parsed as T;
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}

export default streamClient;

/**
 * Parse SSE data line(s) and return the content.
 *
 * SSE format: `"data: <content>"` or multiple `"data: "` lines for multiline
 * content. The SSE spec says `"data:"` is followed by an optional single
 * space, then content. We strip only that mandated leading space and preserve
 * content-internal spaces. Non-SSE lines are treated as NDJSON fallback.
 */
function parseSSEData(event: string): unknown {
  const lines = event.split("\n");
  const dataChunks: string[] = [];

  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed.startsWith("data:")) {
      let content = trimmed.slice(5);
      if (content.startsWith(" ")) {
        content = content.slice(1);
      }
      if (content) {
        dataChunks.push(content);
      }
    } else if (
      trimmed.startsWith("event:") ||
      trimmed.startsWith("id:") ||
      trimmed.startsWith("retry:")
    ) {
      continue;
    } else if (trimmed && !trimmed.startsWith(":")) {
      dataChunks.push(trimmed);
    }
  }

  if (dataChunks.length === 0) {
    return null;
  }

  const data = dataChunks.join("");

  try {
    return JSON.parse(data);
  } catch {
    return { type: "token", content: data };
  }
}
