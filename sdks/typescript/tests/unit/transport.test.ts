/**
 * Unit tests for the transport layer.
 *
 * @module tests/transport.test
 */

import { describe, expect, it, vi } from "vitest";
import {
  BadRequestError,
  InternalError,
  NotFoundError,
} from "../../src/errors.js";
import { FetchTransport } from "../../src/transport/fetch.js";
import {
  createAuthMiddleware,
  createTenantMiddleware,
} from "../../src/transport/middleware.js";
import { createRetryMiddleware } from "../../src/transport/retry.js";
import type { TransportConfig } from "../../src/transport/types.js";

function createMockFetch(responseBody: unknown, status = 200): typeof fetch {
  return vi.fn().mockResolvedValue(
    new Response(JSON.stringify(responseBody), {
      status,
      headers: { "Content-Type": "application/json" },
    }),
  );
}

function createTransportConfig(fetchFn: typeof fetch): TransportConfig {
  return {
    baseUrl: "http://localhost:8080",
    headers: {},
    timeout: 5000,
    maxRetries: 0,
    retryDelay: 100,
    retryStatusCodes: [429, 503],
    fetchFn,
  };
}

describe("FetchTransport", () => {
  it("makes GET requests", async () => {
    const mockFetch = createMockFetch({ status: "healthy" });
    const transport = new FetchTransport(createTransportConfig(mockFetch));

    const result = await transport.request<{ status: string }>({
      method: "GET",
      path: "/health",
    });

    expect(result).toEqual({ status: "healthy" });
    expect(mockFetch).toHaveBeenCalledTimes(1);

    const [url, opts] = (mockFetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe("http://localhost:8080/health");
    expect(opts.method).toBe("GET");
  });

  it("makes POST requests with JSON body", async () => {
    const mockFetch = createMockFetch({ id: "doc-123" });
    const transport = new FetchTransport(createTransportConfig(mockFetch));

    const result = await transport.request<{ id: string }>({
      method: "POST",
      path: "/api/v1/documents",
      body: { content: "hello world" },
    });

    expect(result).toEqual({ id: "doc-123" });

    const [, opts] = (mockFetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.method).toBe("POST");
    expect(opts.headers["Content-Type"]).toBe("application/json");
    expect(JSON.parse(opts.body)).toEqual({ content: "hello world" });
  });

  it("handles 204 No Content", async () => {
    const mockFetch = vi
      .fn()
      .mockResolvedValue(new Response(null, { status: 204 }));
    const transport = new FetchTransport(createTransportConfig(mockFetch));

    const result = await transport.request<void>({
      method: "DELETE",
      path: "/api/v1/documents/123",
    });

    expect(result).toBeUndefined();
  });

  it("throws NotFoundError on 404", async () => {
    const mockFetch = createMockFetch(
      { code: "NOT_FOUND", message: "Document not found" },
      404,
    );
    const transport = new FetchTransport(createTransportConfig(mockFetch));

    await expect(
      transport.request({ method: "GET", path: "/api/v1/documents/missing" }),
    ).rejects.toBeInstanceOf(NotFoundError);
  });

  it("throws BadRequestError on 400", async () => {
    const mockFetch = createMockFetch(
      { code: "BAD_REQUEST", message: "Invalid field" },
      400,
    );
    const transport = new FetchTransport(createTransportConfig(mockFetch));

    await expect(
      transport.request({ method: "POST", path: "/api/v1/documents" }),
    ).rejects.toBeInstanceOf(BadRequestError);
  });

  it("throws InternalError on 500", async () => {
    const mockFetch = createMockFetch(
      { code: "INTERNAL", message: "Server error" },
      500,
    );
    const transport = new FetchTransport(createTransportConfig(mockFetch));

    await expect(
      transport.request({ method: "GET", path: "/health" }),
    ).rejects.toBeInstanceOf(InternalError);
  });

  it("builds WebSocket URL", () => {
    const mockFetch = createMockFetch({});
    const transport = new FetchTransport(createTransportConfig(mockFetch));
    const url = transport.websocketUrl("/ws/pipeline/progress");
    expect(url).toBe("ws://localhost:8080/ws/pipeline/progress");
  });

  it("builds WebSocket URL with HTTPS", () => {
    const mockFetch = createMockFetch({});
    const config = createTransportConfig(mockFetch);
    config.baseUrl = "https://api.example.com";
    const transport = new FetchTransport(config);
    const url = transport.websocketUrl("/ws/pipeline/progress");
    expect(url).toBe("wss://api.example.com/ws/pipeline/progress");
  });

  it("includes query parameters in URL", async () => {
    const mockFetch = createMockFetch({ items: [] });
    const transport = new FetchTransport(createTransportConfig(mockFetch));

    await transport.request({
      method: "GET",
      path: "/api/v1/documents",
      query: { page: 2, per_page: 10, status: "completed" },
    });

    const [url] = (mockFetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toContain("page=2");
    expect(url).toContain("per_page=10");
    expect(url).toContain("status=completed");
  });
});

describe("Auth middleware", () => {
  it("adds API key header", async () => {
    const middleware = createAuthMiddleware({ apiKey: "test-key" });
    const next = vi.fn().mockResolvedValue(new Response());

    const req = { method: "GET" as const, path: "/test" };
    await middleware(req, next);

    const passedReq = next.mock.calls[0][0];
    expect(passedReq.headers["X-API-Key"]).toBe("test-key");
  });

  it("adds Bearer token when no API key", async () => {
    const middleware = createAuthMiddleware({ accessToken: "jwt-token" });
    const next = vi.fn().mockResolvedValue(new Response());

    const req = { method: "GET" as const, path: "/test" };
    await middleware(req, next);

    const passedReq = next.mock.calls[0][0];
    expect(passedReq.headers["Authorization"]).toBe("Bearer jwt-token");
  });

  it("prefers API key over access token", async () => {
    const middleware = createAuthMiddleware({
      apiKey: "key",
      accessToken: "token",
    });
    const next = vi.fn().mockResolvedValue(new Response());

    const req = { method: "GET" as const, path: "/test" };
    await middleware(req, next);

    const passedReq = next.mock.calls[0][0];
    expect(passedReq.headers["X-API-Key"]).toBe("key");
    expect(passedReq.headers["Authorization"]).toBeUndefined();
  });
});

describe("Tenant middleware", () => {
  it("adds tenant, user, and workspace headers", async () => {
    const middleware = createTenantMiddleware({
      tenantId: "t-1",
      userId: "u-1",
      workspaceId: "w-1",
    });
    const next = vi.fn().mockResolvedValue(new Response());

    const req = { method: "GET" as const, path: "/test" };
    await middleware(req, next);

    const passedReq = next.mock.calls[0][0];
    expect(passedReq.headers["X-Tenant-ID"]).toBe("t-1");
    expect(passedReq.headers["X-User-ID"]).toBe("u-1");
    expect(passedReq.headers["X-Workspace-ID"]).toBe("w-1");
  });

  it("skips headers when not configured", async () => {
    const middleware = createTenantMiddleware({});
    const next = vi.fn().mockResolvedValue(new Response());

    const req = { method: "GET" as const, path: "/test" };
    await middleware(req, next);

    const passedReq = next.mock.calls[0][0];
    expect(passedReq.headers["X-Tenant-ID"]).toBeUndefined();
    expect(passedReq.headers["X-User-ID"]).toBeUndefined();
    expect(passedReq.headers["X-Workspace-ID"]).toBeUndefined();
  });
});

describe("Retry middleware", () => {
  it("retries on configured status codes", async () => {
    let callCount = 0;
    const middleware = createRetryMiddleware({
      maxRetries: 2,
      retryDelay: 10,
      retryStatusCodes: [429],
    });

    const next = vi.fn().mockImplementation(async () => {
      callCount++;
      if (callCount < 3) {
        return new Response(null, { status: 429 });
      }
      return new Response(JSON.stringify({ ok: true }), { status: 200 });
    });

    const req = { method: "GET" as const, path: "/test" };
    const response = await middleware(req, next);
    expect(response.status).toBe(200);
    expect(next).toHaveBeenCalledTimes(3);
  });

  it("no retry on non-retryable status", async () => {
    const middleware = createRetryMiddleware({
      maxRetries: 2,
      retryDelay: 10,
      retryStatusCodes: [429, 503],
    });

    const next = vi.fn().mockResolvedValue(new Response(null, { status: 400 }));

    const req = { method: "GET" as const, path: "/test" };
    const response = await middleware(req, next);
    expect(response.status).toBe(400);
    expect(next).toHaveBeenCalledTimes(1);
  });
});
