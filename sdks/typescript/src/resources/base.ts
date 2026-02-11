/**
 * Base resource class that all API resources extend.
 *
 * WHY: Provides common HTTP method helpers (get, post, put, patch, del)
 * so resource classes only need to define the endpoint paths and types.
 * Keeps resource implementations clean and focused on business logic.
 *
 * @module resources/base
 */

import type { HttpTransport, RequestOptions } from "../transport/types.js";

/**
 * Abstract base class for API resource namespaces.
 *
 * Every resource (documents, query, graph, etc.) extends this class
 * and uses the protected helper methods to make typed HTTP requests.
 */
export abstract class Resource {
  constructor(protected readonly transport: HttpTransport) {}

  /** GET request with optional query parameters. */
  protected _get<T>(path: string, query?: Record<string, unknown>): Promise<T> {
    return this.transport.request<T>({
      method: "GET",
      path,
      query: normalizeQuery(query),
    });
  }

  /** POST request with optional JSON body. */
  protected _post<T>(path: string, body?: unknown): Promise<T> {
    return this.transport.request<T>({ method: "POST", path, body });
  }

  /** PUT request with optional JSON body. */
  protected _put<T>(path: string, body?: unknown): Promise<T> {
    return this.transport.request<T>({ method: "PUT", path, body });
  }

  /** PATCH request with optional JSON body. */
  protected _patch<T>(path: string, body?: unknown): Promise<T> {
    return this.transport.request<T>({ method: "PATCH", path, body });
  }

  /** DELETE request. */
  protected _del<T>(path: string): Promise<T> {
    return this.transport.request<T>({ method: "DELETE", path });
  }

  /** SSE streaming request → async iterable of parsed events. */
  protected _streamSSE<T>(
    path: string,
    body?: unknown,
    signal?: AbortSignal,
  ): AsyncIterable<T> {
    const self = this;
    return {
      [Symbol.asyncIterator]() {
        return streamToAsyncIterator<T>(self.transport, path, body, signal);
      },
    };
  }
}

/**
 * Convert a streaming response into an async iterator of parsed JSON events.
 */
async function* streamToAsyncIterator<T>(
  transport: HttpTransport,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
): AsyncIterator<T> & AsyncIterable<T> {
  const options: RequestOptions = body
    ? { method: "POST", path, body, signal }
    : { method: "GET", path, signal };

  for await (const data of transport.stream(options)) {
    try {
      yield JSON.parse(data) as T;
    } catch {
      // WHY: Skip malformed SSE data lines (e.g., comments, empty)
    }
  }
}

/**
 * Normalize query parameters to string values.
 *
 * WHY: fetch() query params must be strings, but callers pass
 * numbers, booleans, etc. for ergonomics.
 */
function normalizeQuery(
  query?: Record<string, unknown>,
): Record<string, string> | undefined {
  if (!query) return undefined;
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined && value !== null) {
      result[key] = String(value);
    }
  }
  return result;
}
