/**
 * Tests for FetchTransport — exercises real transport logic with injected fetchFn.
 *
 * WHY: FetchTransport is the primary HTTP layer; thorough testing ensures
 * URL building, header handling, error mapping, streaming, upload, and
 * timeout all work correctly without hitting a real server.
 */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { NetworkError, TimeoutError } from "../../src/errors.js";
import { FetchTransport } from "../../src/transport/fetch.js";
import type { Middleware, TransportConfig } from "../../src/transport/types.js";

/** Helper to build a minimal TransportConfig with a mock fetch. */
function makeConfig(
  overrides: Partial<TransportConfig> & { fetchFn: typeof fetch },
): TransportConfig {
  return {
    baseUrl: "http://localhost:8080",
    headers: { Authorization: "Bearer test-token" },
    timeout: 5000,
    maxRetries: 0,
    retryDelay: 100,
    retryStatusCodes: [429, 503],
    ...overrides,
  };
}

/** Helper to create a mock Response object. */
function mockResponse(
  body: unknown,
  init?: {
    status?: number;
    statusText?: string;
    headers?: Record<string, string>;
  },
): Response {
  const status = init?.status ?? 200;
  const jsonBody = JSON.stringify(body);
  return new Response(jsonBody, {
    status,
    statusText: init?.statusText ?? "OK",
    headers: { "Content-Type": "application/json", ...init?.headers },
  });
}

/** Create a 204 No Content response. */
function noContentResponse(): Response {
  return new Response(null, { status: 204, statusText: "No Content" });
}

describe("FetchTransport", () => {
  let mockFetch: ReturnType<typeof vi.fn>;
  let transport: FetchTransport;

  beforeEach(() => {
    mockFetch = vi.fn();
    transport = new FetchTransport(
      makeConfig({ fetchFn: mockFetch as unknown as typeof fetch }),
    );
  });

  // ─────────────── request() ───────────────

  describe("request()", () => {
    it("sends JSON request with correct URL and headers", async () => {
      mockFetch.mockResolvedValue(mockResponse({ id: "1" }));

      const result = await transport.request<{ id: string }>({
        method: "GET",
        path: "/api/v1/documents",
      });

      expect(result).toEqual({ id: "1" });
      expect(mockFetch).toHaveBeenCalledTimes(1);

      const [url, init] = mockFetch.mock.calls[0];
      expect(url).toBe("http://localhost:8080/api/v1/documents");
      expect(init.method).toBe("GET");
      expect(init.headers.Authorization).toBe("Bearer test-token");
    });

    it("sends POST with JSON body", async () => {
      mockFetch.mockResolvedValue(mockResponse({ created: true }));

      await transport.request({
        method: "POST",
        path: "/api/v1/documents",
        body: { title: "test" },
      });

      const [, init] = mockFetch.mock.calls[0];
      expect(init.headers["Content-Type"]).toBe("application/json");
      expect(init.body).toBe(JSON.stringify({ title: "test" }));
    });

    it("handles 204 No Content as undefined", async () => {
      mockFetch.mockResolvedValue(noContentResponse());

      const result = await transport.request({
        method: "DELETE",
        path: "/api/v1/documents/1",
      });

      expect(result).toBeUndefined();
    });

    it("builds URL with query params", async () => {
      mockFetch.mockResolvedValue(mockResponse({ items: [] }));

      await transport.request({
        method: "GET",
        path: "/api/v1/documents",
        query: { limit: 10, offset: 0, active: true, missing: undefined },
      });

      const [url] = mockFetch.mock.calls[0];
      const parsed = new URL(url);
      expect(parsed.searchParams.get("limit")).toBe("10");
      expect(parsed.searchParams.get("offset")).toBe("0");
      expect(parsed.searchParams.get("active")).toBe("true");
      expect(parsed.searchParams.has("missing")).toBe(false);
    });

    it("merges custom request headers", async () => {
      mockFetch.mockResolvedValue(mockResponse({}));

      await transport.request({
        method: "GET",
        path: "/test",
        headers: { "X-Custom": "foo" },
      });

      const [, init] = mockFetch.mock.calls[0];
      expect(init.headers["X-Custom"]).toBe("foo");
      expect(init.headers.Authorization).toBe("Bearer test-token");
    });

    it("throws typed error on 4xx response", async () => {
      mockFetch.mockResolvedValue(
        mockResponse(
          { code: "NOT_FOUND", message: "Not found" },
          { status: 404 },
        ),
      );

      await expect(
        transport.request({ method: "GET", path: "/missing" }),
      ).rejects.toThrow("Not found");
    });

    it("throws typed error on 5xx response", async () => {
      mockFetch.mockResolvedValue(
        mockResponse(
          { code: "INTERNAL", message: "Server error" },
          { status: 500 },
        ),
      );

      await expect(
        transport.request({ method: "GET", path: "/broken" }),
      ).rejects.toThrow("Server error");
    });

    it("handles non-JSON error response bodies", async () => {
      mockFetch.mockResolvedValue(
        new Response("Internal Server Error", {
          status: 500,
          statusText: "Internal Server Error",
        }),
      );

      await expect(
        transport.request({ method: "GET", path: "/broken" }),
      ).rejects.toThrow();
    });

    it("throws NetworkError on TypeError (network failure)", async () => {
      mockFetch.mockRejectedValue(new TypeError("fetch failed"));

      await expect(
        transport.request({ method: "GET", path: "/down" }),
      ).rejects.toBeInstanceOf(NetworkError);
    });

    it("throws TimeoutError when request exceeds timeout", async () => {
      // Use a tiny timeout and slow fetch
      const slowTransport = new FetchTransport(
        makeConfig({
          fetchFn: ((_url: string, init: RequestInit) => {
            return new Promise((_resolve, _reject) => {
              // Simulate slow response — just abort after controller fires
              if (init.signal) {
                init.signal.addEventListener("abort", () => {
                  _reject(
                    new DOMException(
                      "The operation was aborted.",
                      "AbortError",
                    ),
                  );
                });
              }
            });
          }) as unknown as typeof fetch,
          timeout: 50,
        }),
      );

      await expect(
        slowTransport.request({ method: "GET", path: "/slow" }),
      ).rejects.toBeInstanceOf(TimeoutError);
    });

    it("re-throws non-TypeError/non-AbortError errors", async () => {
      mockFetch.mockRejectedValue(new Error("custom error"));

      await expect(
        transport.request({ method: "GET", path: "/errored" }),
      ).rejects.toThrow("custom error");
    });
  });

  // ─────────────── stream() ───────────────

  describe("stream()", () => {
    it("yields SSE data lines", async () => {
      const sseBody = `data: {"token":"Hello"}\n\ndata: {"token":" world"}\n\ndata: [DONE]\n\n`;
      mockFetch.mockResolvedValue(new Response(sseBody, { status: 200 }));

      const chunks: string[] = [];
      for await (const line of transport.stream({
        method: "GET",
        path: "/stream",
      })) {
        chunks.push(line);
      }

      expect(chunks).toEqual(['{"token":"Hello"}', '{"token":" world"}']);
    });

    it("throws if body is null", async () => {
      // Create a Response with no body by using Response.redirect workaround
      const response = new Response(null, { status: 200 });
      Object.defineProperty(response, "body", { value: null });
      mockFetch.mockResolvedValue(response);

      const gen = transport.stream({ method: "GET", path: "/no-body" });
      await expect(gen[Symbol.asyncIterator]().next()).rejects.toThrow(
        "streaming unavailable",
      );
    });

    it("handles multi-chunk delivery", async () => {
      // Simulate data split across chunks (partial SSE event)
      const encoder = new TextEncoder();
      const chunks = [
        encoder.encode('data: {"a":'),
        encoder.encode("1}\n\ndata: [DONE]\n\n"),
      ];

      let chunkIndex = 0;
      const readableStream = new ReadableStream({
        pull(controller) {
          if (chunkIndex < chunks.length) {
            controller.enqueue(chunks[chunkIndex++]);
          } else {
            controller.close();
          }
        },
      });

      mockFetch.mockResolvedValue(
        new Response(readableStream, { status: 200 }),
      );

      const lines: string[] = [];
      for await (const line of transport.stream({
        method: "POST",
        path: "/stream",
      })) {
        lines.push(line);
      }

      expect(lines).toEqual(['{"a":1}']);
    });
  });

  // ─────────────── upload() ───────────────

  describe("upload()", () => {
    it("sends file as FormData", async () => {
      mockFetch.mockResolvedValue(mockResponse({ id: "upload-1" }));

      const file = new Blob(["hello"], { type: "text/plain" });
      const result = await transport.upload<{ id: string }>(
        "/api/v1/documents",
        file,
        { title: "test.txt" },
      );

      expect(result.id).toBe("upload-1");
      const [, init] = mockFetch.mock.calls[0];
      expect(init.body).toBeInstanceOf(FormData);
      // WHY: Content-Type should NOT be set for FormData — browser sets boundary
      expect(init.headers?.["Content-Type"]).toBeUndefined();
    });

    it("converts ArrayBuffer to Blob", async () => {
      mockFetch.mockResolvedValue(mockResponse({ id: "upload-2" }));

      const buffer = new ArrayBuffer(4);
      await transport.upload("/api/v1/documents", buffer);

      const [, init] = mockFetch.mock.calls[0];
      expect(init.body).toBeInstanceOf(FormData);
    });

    it("converts Uint8Array to Blob", async () => {
      mockFetch.mockResolvedValue(mockResponse({ id: "upload-3" }));

      const bytes = new Uint8Array([1, 2, 3, 4]);
      await transport.upload("/api/v1/documents", bytes);

      const [, init] = mockFetch.mock.calls[0];
      expect(init.body).toBeInstanceOf(FormData);
    });
  });

  // ─────────────── uploadBatch() ───────────────

  describe("uploadBatch()", () => {
    it("sends multiple files as FormData", async () => {
      mockFetch.mockResolvedValue(mockResponse({ count: 2 }));

      const files = [
        new Blob(["a"], { type: "text/plain" }),
        new Blob(["b"], { type: "text/plain" }),
      ];

      const result = await transport.uploadBatch<{ count: number }>(
        "/api/v1/documents/batch",
        files,
        { workspace: "default" },
      );

      expect(result.count).toBe(2);
      const [, init] = mockFetch.mock.calls[0];
      expect(init.body).toBeInstanceOf(FormData);
    });
  });

  // ─────────────── requestBlob() ───────────────

  describe("requestBlob()", () => {
    it("returns Blob from response", async () => {
      const blobBody = new Blob(["pdf-content"], { type: "application/pdf" });
      mockFetch.mockResolvedValue(new Response(blobBody, { status: 200 }));

      const result = await transport.requestBlob({
        method: "GET",
        path: "/api/v1/documents/1/download",
      });

      expect(result).toBeInstanceOf(Blob);
      const text = await result.text();
      expect(text).toBe("pdf-content");
    });
  });

  // ─────────────── websocketUrl() ───────────────

  describe("websocketUrl()", () => {
    it("replaces http with ws", () => {
      const url = transport.websocketUrl("/ws/pipeline/progress");
      expect(url).toBe("ws://localhost:8080/ws/pipeline/progress");
    });

    it("replaces https with wss", () => {
      const httpsTransport = new FetchTransport(
        makeConfig({
          fetchFn: mockFetch as unknown as typeof fetch,
          baseUrl: "https://api.example.com",
        }),
      );
      const url = httpsTransport.websocketUrl("/ws/test");
      expect(url).toBe("wss://api.example.com/ws/test");
    });
  });

  // ─────────────── middleware ───────────────

  describe("middleware", () => {
    it("applies middleware in order", async () => {
      const order: string[] = [];

      const mw1: Middleware = async (req, next) => {
        order.push("mw1-before");
        const res = await next(req);
        order.push("mw1-after");
        return res;
      };

      const mw2: Middleware = async (req, next) => {
        order.push("mw2-before");
        const res = await next(req);
        order.push("mw2-after");
        return res;
      };

      mockFetch.mockResolvedValue(mockResponse({ ok: true }));

      const mwTransport = new FetchTransport(
        makeConfig({ fetchFn: mockFetch as unknown as typeof fetch }),
        [mw1, mw2],
      );

      await mwTransport.request({ method: "GET", path: "/test" });

      expect(order).toEqual([
        "mw1-before",
        "mw2-before",
        "mw2-after",
        "mw1-after",
      ]);
    });
  });

  // ─────────────── combineSignals ───────────────

  describe("signal handling", () => {
    it("propagates user AbortSignal", async () => {
      const controller = new AbortController();
      controller.abort();

      mockFetch.mockImplementation((_url: string, init: RequestInit) => {
        if (init.signal?.aborted) {
          return Promise.reject(
            new DOMException("The operation was aborted.", "AbortError"),
          );
        }
        return Promise.resolve(mockResponse({}));
      });

      // User signal is already aborted, so the request should fail
      await expect(
        transport.request({
          method: "GET",
          path: "/test",
          signal: controller.signal,
        }),
      ).rejects.toThrow();
    });
  });
});
