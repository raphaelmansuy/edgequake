/**
 * SPEC-018: WebUI observability — correlation headers + explicit error context.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

describe("SPEC-018 observability client", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("sets traceparent on apiClient fetch", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue({
      ok: true,
      headers: new Headers(),
      text: async () => JSON.stringify({ status: "healthy" }),
    } as Response);

    process.env.EDGEQUAKE_API_URL = "http://backend.test:8080/api/v1";
    const { api } = await import("../client");
    await api.get("/health");

    const init = fetchMock.mock.calls[0][1] as RequestInit;
    const headers = new Headers(init.headers);
    const tp = headers.get("traceparent");
    expect(tp).toMatch(/^00-[0-9a-f]{32}-[0-9a-f]{16}-01$/);
  });

  it("sets X-Request-ID on apiClient fetch", async () => {
    const fetchMock = vi.spyOn(globalThis, "fetch").mockResolvedValue({
      ok: true,
      headers: new Headers(),
      text: async () => JSON.stringify({ status: "healthy" }),
    } as Response);

    process.env.EDGEQUAKE_API_URL = "http://backend.test:8080/api/v1";
    const { api } = await import("../client");
    await api.get("/health");

    expect(fetchMock).toHaveBeenCalled();
    const init = fetchMock.mock.calls[0][1] as RequestInit;
    const headers = new Headers(init.headers);
    const requestId = headers.get("X-Request-ID");
    expect(requestId).toBeTruthy();
    expect(requestId!.length).toBeGreaterThan(8);
  });

  it("surfaces backend error details on failed responses", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    vi.spyOn(globalThis, "fetch").mockResolvedValue({
      ok: false,
      status: 404,
      statusText: "Not Found",
      url: "http://backend.test:8080/api/v1/documents/missing",
      headers: new Headers({ "x-request-id": "resp-req-42" }),
      json: async () => ({
        code: "NOT_FOUND",
        message: "Document not found: doc-1",
        details: {
          request_id: "body-req-99",
          error_code: "NOT_FOUND",
          retryable: false,
          source: "api",
          diagnostics: { kind: "not_found", resource: "doc-1" },
        },
      }),
    } as Response);

    process.env.EDGEQUAKE_API_URL = "http://backend.test:8080/api/v1";
    const { api, apiErrorLogPayload } = await import("../client");

    const err = (await api.get("/documents/missing").catch((e) => e)) as import("../client").ApiRequestError;
    expect(err).toMatchObject({
      status: 404,
      code: "NOT_FOUND",
    });

    expect(warnSpy).toHaveBeenCalled();
    const payload = warnSpy.mock.calls[0][1] as Record<string, unknown>;
    expect(payload.request_id).toBe("resp-req-42");
    expect(payload.url).toBe("http://backend.test:8080/api/v1/documents/missing");
    expect(payload.diagnostics).toEqual({
      kind: "not_found",
      resource: "doc-1",
    });

    const logPayload = apiErrorLogPayload(err);
    expect(logPayload.retryable).toBe(false);
    expect(logPayload.source).toBe("api");
    expect(logPayload).toHaveProperty("traceparent");
    expect(logPayload).toHaveProperty("trace_id");
  });

  it("logs 5xx API errors with warn (not error) for Next.js dev overlay safety", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.spyOn(globalThis, "fetch").mockResolvedValue({
      ok: false,
      status: 500,
      statusText: "Internal Server Error",
      url: "http://backend.test:8080/api/v1/conversations",
      headers: new Headers(),
      json: async () => ({}),
    } as Response);

    process.env.EDGEQUAKE_API_URL = "http://backend.test:8080/api/v1";
    const { api } = await import("../client");

    await expect(api.get("/conversations")).rejects.toMatchObject({ status: 500 });

    expect(warnSpy).toHaveBeenCalled();
    expect(errorSpy).not.toHaveBeenCalled();
    const payload = warnSpy.mock.calls[0][1] as Record<string, unknown>;
    expect(payload.status).toBe(500);
    expect(payload.message).toBe("(no message)");
    expect(payload.url).toContain("/conversations");
  });

  it("logs network errors with trace context (warn, not error)", async () => {
    // WHY console.warn: Next.js dev promotes console.error to a full-screen
    // overlay, which crashes the dashboard on backend cold start. Network
    // errors are still thrown to callers; the log level only controls dev UX.
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new TypeError("fetch failed"));

    process.env.EDGEQUAKE_API_URL = "http://backend.test:8080/api/v1";
    const { api } = await import("../client");

    await expect(api.get("/health")).rejects.toThrow("Network request failed");
    expect(warnSpy).toHaveBeenCalled();
    expect(errorSpy).not.toHaveBeenCalled();
    const payload = warnSpy.mock.calls[0][1] as Record<string, unknown>;
    expect(payload.code).toBe("NETWORK_ERROR");
    expect(payload.source).toBe("webui_client");
    expect(payload).toHaveProperty("trace_id");
  });

  it("silent apiClient requests suppress warn/error (dev overlay safe)", async () => {
    const warnSpy = vi.spyOn(console, "warn").mockImplementation(() => {});
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const debugSpy = vi.spyOn(console, "debug").mockImplementation(() => {});
    vi.spyOn(globalThis, "fetch").mockRejectedValue(new TypeError("fetch failed"));

    process.env.EDGEQUAKE_API_URL = "http://backend.test:8080/api/v1";
    const { api } = await import("../client");

    await expect(
      api.get("/health", { silent: true }),
    ).rejects.toThrow("Network request failed");

    expect(warnSpy).not.toHaveBeenCalled();
    expect(errorSpy).not.toHaveBeenCalled();
    expect(debugSpy).toHaveBeenCalled();
  });
});
