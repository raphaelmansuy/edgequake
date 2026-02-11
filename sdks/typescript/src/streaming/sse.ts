/**
 * SSE (Server-Sent Events) streaming utilities.
 *
 * WHY: SSE is used for query streaming and chat streaming in EdgeQuake.
 * This parser handles the SSE wire format (data: lines, [DONE] sentinel).
 *
 * @module streaming/sse
 */

/**
 * Parse an SSE response body into an async iterable of typed events.
 *
 * Handles:
 * - `data: {...json...}` lines → parsed as T
 * - `data: [DONE]` → ends iteration
 * - Empty lines / comments → skipped
 *
 * @param response - Fetch Response with streaming body
 * @param parser - Function to parse raw data string into T (return null to skip)
 * @param signal - Optional AbortSignal for cancellation
 */
export async function* parseSSEStream<T>(
  response: Response,
  parser: (raw: string) => T | null,
  signal?: AbortSignal,
): AsyncGenerator<T> {
  if (!response.body) {
    throw new Error("Response body is null — SSE streaming unavailable");
  }

  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  try {
    while (true) {
      if (signal?.aborted) {
        throw new DOMException("Aborted", "AbortError");
      }

      const { done, value } = await reader.read();
      if (done) break;

      buffer += decoder.decode(value, { stream: true });

      // WHY: SSE events are separated by double newlines
      while (true) {
        const eventEnd = buffer.indexOf("\n\n");
        if (eventEnd === -1) break;

        const eventText = buffer.slice(0, eventEnd);
        buffer = buffer.slice(eventEnd + 2);

        for (const line of eventText.split("\n")) {
          if (line.startsWith("data: ")) {
            const data = line.slice(6);
            if (data === "[DONE]") return;

            const parsed = parser(data);
            if (parsed !== null) yield parsed;
          }
          // WHY: Lines starting with ":" are SSE comments — ignore
        }
      }
    }
  } finally {
    reader.releaseLock();
  }
}
