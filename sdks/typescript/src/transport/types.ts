/**
 * Transport layer type definitions.
 *
 * WHY: Separating types from implementation enables testing with mock
 * transports and future transport backends (e.g., WebSocket-only).
 *
 * @module transport/types
 */

/** HTTP method verbs supported by the transport. */
export type HttpMethod = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

/** Options for a single HTTP request. */
export interface RequestOptions {
  method: HttpMethod;
  path: string;
  body?: unknown;
  query?: Record<string, string | number | boolean | undefined>;
  headers?: Record<string, string>;
  signal?: AbortSignal;
  timeout?: number;
}

/**
 * Transport configuration.
 *
 * WHY: All config is resolved once at construction time so individual
 * requests carry zero allocation overhead for header/URL building.
 */
export interface TransportConfig {
  baseUrl: string;
  headers: Record<string, string>;
  timeout: number;
  maxRetries: number;
  retryDelay: number;
  retryStatusCodes: number[];
  fetchFn: typeof fetch;
}

/**
 * Abstract HTTP transport interface.
 *
 * WHY: Resources depend on this interface (not FetchTransport directly),
 * enabling mock transports for unit testing without network calls.
 */
export interface HttpTransport {
  /** JSON request → parsed response. */
  request<T>(options: RequestOptions): Promise<T>;

  /** Streaming request → async iterable of raw SSE data lines. */
  stream(options: RequestOptions): AsyncIterable<string>;

  /** File upload via multipart/form-data. */
  upload<T>(
    path: string,
    file: Blob | ArrayBuffer | Uint8Array,
    metadata?: Record<string, string>,
  ): Promise<T>;

  /** Batch file upload via multipart/form-data. */
  uploadBatch<T>(
    path: string,
    files: Array<Blob | ArrayBuffer | Uint8Array>,
    metadata?: Record<string, string>,
  ): Promise<T>;

  /** Binary download → Blob. */
  requestBlob(options: RequestOptions): Promise<Blob>;

  /** Build a WebSocket URL for the given path. */
  websocketUrl(path: string): string;
}

/**
 * Middleware function type.
 *
 * WHY: Middleware pattern allows composable request/response processing
 * (auth headers, tenant headers, logging) without coupling to transport.
 */
export type Middleware = (
  request: RequestOptions,
  next: (request: RequestOptions) => Promise<Response>,
) => Promise<Response>;
