/**
 * Fetch-based HTTP transport.
 *
 * WHY: Uses native fetch() (Node 18+, Deno, Bun, browser) for zero
 * external dependencies. All HTTP communication flows through this class.
 *
 * @module transport/fetch
 */

import {
  NetworkError,
  TimeoutError,
  parseErrorResponse,
  type ErrorResponseBody,
} from "../errors.js";
import type {
  HttpTransport,
  Middleware,
  RequestOptions,
  TransportConfig,
} from "./types.js";

/**
 * HTTP transport implementation using native fetch.
 *
 * Supports: JSON requests, SSE streaming, multipart uploads, blob downloads.
 * Applies middleware stack (auth → tenant → retry) to every request.
 */
export class FetchTransport implements HttpTransport {
  private readonly config: TransportConfig;
  private readonly middlewares: Middleware[];

  constructor(config: TransportConfig, middlewares: Middleware[] = []) {
    this.config = config;
    this.middlewares = middlewares;
  }

  /** JSON request → parsed response body. */
  async request<T>(options: RequestOptions): Promise<T> {
    const response = await this.executeWithMiddleware(options);
    await this.checkResponseStatus(response);

    // WHY: Some DELETE endpoints return 204 No Content
    if (response.status === 204) {
      return undefined as T;
    }

    // WHY: Some endpoints (e.g., /ready, /live) return plain text "OK"
    // instead of JSON. Detect via Content-Type header to avoid parse errors.
    const contentType = response.headers.get("content-type") ?? "";
    if (!contentType.includes("application/json")) {
      const text = await response.text();
      return text as T;
    }

    return (await response.json()) as T;
  }

  /** Streaming request → async iterable of raw SSE `data:` lines. */
  async *stream(options: RequestOptions): AsyncIterable<string> {
    const response = await this.executeWithMiddleware(options);
    await this.checkResponseStatus(response);

    if (!response.body) {
      throw new NetworkError("Response body is null — streaming unavailable");
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = "";

    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });

        // WHY: SSE events are separated by double newline
        while (true) {
          const eventEnd = buffer.indexOf("\n\n");
          if (eventEnd === -1) break;

          const eventText = buffer.slice(0, eventEnd);
          buffer = buffer.slice(eventEnd + 2);

          for (const line of eventText.split("\n")) {
            if (line.startsWith("data: ")) {
              const data = line.slice(6);
              if (data === "[DONE]") return;
              yield data;
            }
          }
        }
      }
    } finally {
      reader.releaseLock();
    }
  }

  /** File upload via multipart/form-data. */
  async upload<T>(
    path: string,
    file: Blob | ArrayBuffer | Uint8Array,
    metadata?: Record<string, string>,
  ): Promise<T> {
    const formData = new FormData();

    // WHY: Convert Buffer/ArrayBuffer to Blob for universal FormData compatibility
    const blob = file instanceof Blob ? file : new Blob([file as BlobPart]);
    formData.append("file", blob);

    if (metadata) {
      for (const [key, value] of Object.entries(metadata)) {
        formData.append(key, value);
      }
    }

    const response = await this.executeWithMiddleware({
      method: "POST",
      path,
      body: formData,
    });

    await this.checkResponseStatus(response);
    return (await response.json()) as T;
  }

  /** Batch file upload via multipart/form-data. */
  async uploadBatch<T>(
    path: string,
    files: Array<Blob | ArrayBuffer | Uint8Array>,
    metadata?: Record<string, string>,
  ): Promise<T> {
    const formData = new FormData();

    for (const file of files) {
      const blob = file instanceof Blob ? file : new Blob([file as BlobPart]);
      formData.append("files", blob);
    }

    if (metadata) {
      for (const [key, value] of Object.entries(metadata)) {
        formData.append(key, value);
      }
    }

    const response = await this.executeWithMiddleware({
      method: "POST",
      path,
      body: formData,
    });

    await this.checkResponseStatus(response);
    return (await response.json()) as T;
  }

  /** Binary download → Blob. */
  async requestBlob(options: RequestOptions): Promise<Blob> {
    const response = await this.executeWithMiddleware(options);
    await this.checkResponseStatus(response);
    return response.blob();
  }

  /** Build a WebSocket URL for the given path. */
  websocketUrl(path: string): string {
    const base = this.config.baseUrl.replace(/^http/, "ws");
    return `${base}${path}`;
  }

  // ── Private Helpers ──────────────────────────────────────────

  /**
   * Execute a request through the middleware stack.
   *
   * WHY: Middleware is composed inside-out — the last middleware wraps
   * closest to the actual fetch call, ensuring auth runs first.
   */
  private async executeWithMiddleware(
    options: RequestOptions,
  ): Promise<Response> {
    // Build the base fetch function
    type NextFn = (req: RequestOptions) => Promise<Response>;
    const baseFetch: NextFn = (req: RequestOptions) => this.rawFetch(req);

    // WHY: Compose middlewares right-to-left so first middleware runs first
    const handler: NextFn = this.middlewares.reduceRight<NextFn>(
      (next: NextFn, middleware) => (req: RequestOptions) =>
        middleware(req, next),
      baseFetch,
    );

    return handler(options);
  }

  /**
   * Raw fetch call with URL building, headers, timeout.
   */
  private async rawFetch(options: RequestOptions): Promise<Response> {
    const url = this.buildUrl(options.path, options.query);
    const timeout = options.timeout ?? this.config.timeout;

    // WHY: AbortController enables request timeout without external deps
    const controller = new AbortController();
    const signal = options.signal
      ? this.combineSignals(options.signal, controller.signal)
      : controller.signal;

    const timeoutId = timeout
      ? setTimeout(() => controller.abort(), timeout)
      : undefined;

    try {
      const isFormData = options.body instanceof FormData;

      const headers: Record<string, string> = {
        ...this.config.headers,
        ...options.headers,
      };

      // WHY: Don't set Content-Type for FormData — browser/node sets
      // multipart boundary automatically
      if (!isFormData && options.body !== undefined) {
        headers["Content-Type"] = "application/json";
      }

      const response = await this.config.fetchFn(url, {
        method: options.method,
        headers,
        body: isFormData
          ? (options.body as FormData)
          : options.body !== undefined
            ? JSON.stringify(options.body)
            : undefined,
        signal,
      });

      return response;
    } catch (error) {
      if (
        error instanceof DOMException &&
        error.name === "AbortError" &&
        !options.signal?.aborted
      ) {
        throw new TimeoutError(`Request timed out after ${timeout}ms`);
      }
      if (error instanceof TypeError) {
        throw new NetworkError(`Network error: ${error.message}`, error);
      }
      throw error;
    } finally {
      if (timeoutId) clearTimeout(timeoutId);
    }
  }

  /**
   * Build full URL from base URL + path + query params.
   */
  private buildUrl(
    path: string,
    query?: Record<string, string | number | boolean | undefined>,
  ): string {
    const url = new URL(path, this.config.baseUrl);

    if (query) {
      for (const [key, value] of Object.entries(query)) {
        if (value !== undefined && value !== null) {
          url.searchParams.set(key, String(value));
        }
      }
    }

    return url.toString();
  }

  /**
   * Check HTTP response status and throw typed errors.
   */
  private async checkResponseStatus(response: Response): Promise<void> {
    if (response.ok) return;

    let body: ErrorResponseBody;
    try {
      body = (await response.json()) as ErrorResponseBody;
    } catch {
      // WHY: Some error responses may not be valid JSON
      body = {
        code: "UNKNOWN",
        message: response.statusText || `HTTP ${response.status}`,
      };
    }

    throw parseErrorResponse(response.status, body);
  }

  /**
   * Combine two AbortSignals — abort if either fires.
   *
   * WHY: User may pass their own signal (e.g., React unmount) AND we
   * add a timeout signal. Both should cancel the request.
   */
  private combineSignals(
    userSignal: AbortSignal,
    timeoutSignal: AbortSignal,
  ): AbortSignal {
    // WHY: AbortSignal.any() is available in Node 20+, fall back for 18
    if ("any" in AbortSignal) {
      return AbortSignal.any([userSignal, timeoutSignal]);
    }

    const controller = new AbortController();
    const onAbort = () => controller.abort();
    userSignal.addEventListener("abort", onAbort);
    timeoutSignal.addEventListener("abort", onAbort);
    return controller.signal;
  }
}
