/**
 * Mock transport for unit testing resource classes.
 *
 * WHY: Allows testing every resource method without HTTP I/O.
 * Records requests for assertion and returns canned responses.
 *
 * @module tests/helpers/mock-transport
 */

import type {
  HttpTransport,
  RequestOptions,
} from "../../src/transport/types.js";

/** Recorded request for assertion. */
export interface RecordedRequest {
  method: string;
  path: string;
  body?: unknown;
  query?: Record<string, string>;
}

/** Route handler — maps "METHOD /path" to a response. */
export interface MockRoute {
  status?: number;
  body?: unknown;
  /** For stream() — array of string chunks to yield. */
  chunks?: string[];
  /** For requestBlob() — blob content. */
  blob?: Blob;
}

/**
 * Create a mock transport that records requests and returns configured responses.
 *
 * @example
 * ```ts
 * const mock = createMockTransport({
 *   "GET /health": { body: { status: "healthy" } },
 *   "POST /api/v1/documents": { body: { document_id: "doc-1" } },
 * });
 * ```
 */
export function createMockTransport(
  routes: Record<string, MockRoute> = {},
): HttpTransport & {
  requests: RecordedRequest[];
  lastRequest: RecordedRequest | undefined;
} {
  const requests: RecordedRequest[] = [];

  function findRoute(method: string, path: string): MockRoute {
    // Exact match first
    const key = `${method} ${path}`;
    if (routes[key]) return routes[key];
    // Try without query string
    const pathOnly = path.split("?")[0];
    const keyNoQuery = `${method} ${pathOnly}`;
    if (routes[keyNoQuery]) return routes[keyNoQuery];
    // Default: return empty body
    return { status: 200, body: {} };
  }

  function record(opts: RequestOptions): void {
    requests.push({
      method: opts.method,
      path: opts.path,
      body: opts.body,
      query: opts.query,
    });
  }

  return {
    requests,
    get lastRequest() {
      return requests[requests.length - 1];
    },

    async request<T>(options: RequestOptions): Promise<T> {
      record(options);
      const route = findRoute(options.method, options.path);
      if (route.status && route.status >= 400) {
        const { parseErrorResponse } = await import("../../src/errors.js");
        throw parseErrorResponse(
          route.status,
          route.body as Record<string, unknown> | undefined,
        );
      }
      return route.body as T;
    },

    async *stream(options: RequestOptions): AsyncIterable<string> {
      record(options);
      const route = findRoute(options.method, options.path);
      if (route.chunks) {
        for (const chunk of route.chunks) {
          yield chunk;
        }
      }
    },

    async upload<T>(
      path: string,
      file: File | Blob,
      metadata?: Record<string, string>,
    ): Promise<T> {
      requests.push({ method: "POST", path, body: { _file: true, metadata } });
      const route = findRoute("POST", path);
      return route.body as T;
    },

    async uploadBatch<T>(path: string, files: (File | Blob)[]): Promise<T> {
      requests.push({ method: "POST", path, body: { _files: files.length } });
      const route = findRoute("POST", path);
      return route.body as T;
    },

    async requestBlob(options: RequestOptions): Promise<Blob> {
      record(options);
      const route = findRoute(options.method, options.path);
      return route.blob ?? new Blob(["mock-blob"]);
    },

    websocketUrl(path: string): string {
      return `ws://mock${path}`;
    },
  };
}
