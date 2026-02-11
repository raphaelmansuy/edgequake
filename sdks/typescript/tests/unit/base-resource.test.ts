/**
 * Base resource tests — normalizeQuery, _streamSSE, HTTP helpers.
 */

import { describe, expect, it } from "vitest";
import { Resource } from "../../src/resources/base.js";
import type { HttpTransport } from "../../src/transport/types.js";
import { createMockTransport } from "../helpers/mock-transport.js";

// Concrete subclass for testing abstract Resource
class TestResource extends Resource {
  get(path: string, query?: Record<string, unknown>) {
    return this._get(path, query);
  }
  post(path: string, body?: unknown) {
    return this._post(path, body);
  }
  put(path: string, body?: unknown) {
    return this._put(path, body);
  }
  patch(path: string, body?: unknown) {
    return this._patch(path, body);
  }
  del(path: string) {
    return this._del(path);
  }
  streamSSE<T>(path: string, body?: unknown, signal?: AbortSignal) {
    return this._streamSSE<T>(path, body, signal);
  }
}

describe("Resource base class", () => {
  let mock: ReturnType<typeof createMockTransport>;
  let resource: TestResource;

  function setup(
    routes: Record<string, { body?: unknown; chunks?: string[] }> = {},
  ) {
    mock = createMockTransport(routes);
    resource = new TestResource(mock as unknown as HttpTransport);
  }

  // ─── HTTP helpers ───

  it("_get sends GET request", async () => {
    setup({ "GET /foo": { body: { ok: true } } });
    const result = await resource.get("/foo");
    expect(result).toEqual({ ok: true });
    expect(mock.lastRequest?.method).toBe("GET");
    expect(mock.lastRequest?.path).toBe("/foo");
  });

  it("_get with query params normalizes values", async () => {
    setup({ "GET /foo": { body: {} } });
    await resource.get("/foo", {
      page: 1,
      active: true,
      name: "test",
      empty: null,
      undef: undefined,
    });
    expect(mock.lastRequest?.query).toEqual({
      page: "1",
      active: "true",
      name: "test",
    });
  });

  it("_get with no query passes undefined", async () => {
    setup({ "GET /bar": { body: {} } });
    await resource.get("/bar");
    expect(mock.lastRequest?.query).toBeUndefined();
  });

  it("_post sends POST with body", async () => {
    setup({ "POST /items": { body: { id: 1 } } });
    const result = await resource.post("/items", { name: "test" });
    expect(result).toEqual({ id: 1 });
    expect(mock.lastRequest?.method).toBe("POST");
    expect(mock.lastRequest?.body).toEqual({ name: "test" });
  });

  it("_post with no body", async () => {
    setup({ "POST /action": { body: {} } });
    await resource.post("/action");
    expect(mock.lastRequest?.body).toBeUndefined();
  });

  it("_put sends PUT with body", async () => {
    setup({ "PUT /items/1": { body: { updated: true } } });
    const result = await resource.put("/items/1", { name: "updated" });
    expect(result).toEqual({ updated: true });
    expect(mock.lastRequest?.method).toBe("PUT");
  });

  it("_patch sends PATCH with body", async () => {
    setup({ "PATCH /items/1": { body: { patched: true } } });
    const result = await resource.patch("/items/1", { field: "value" });
    expect(result).toEqual({ patched: true });
    expect(mock.lastRequest?.method).toBe("PATCH");
  });

  it("_del sends DELETE", async () => {
    setup({ "DELETE /items/1": { body: {} } });
    await resource.del("/items/1");
    expect(mock.lastRequest?.method).toBe("DELETE");
    expect(mock.lastRequest?.path).toBe("/items/1");
  });

  // ─── Streaming ───

  it("_streamSSE yields parsed JSON from stream chunks", async () => {
    setup({
      "POST /stream": {
        chunks: ['{"type":"data","value":1}', '{"type":"data","value":2}'],
      },
    });

    const events: Array<{ type: string; value: number }> = [];
    for await (const ev of resource.streamSSE<{ type: string; value: number }>(
      "/stream",
      { query: "test" },
    )) {
      events.push(ev);
    }
    expect(events).toEqual([
      { type: "data", value: 1 },
      { type: "data", value: 2 },
    ]);
  });

  it("_streamSSE with no body uses GET", async () => {
    setup({
      "GET /events": { chunks: ['{"ok":true}'] },
    });

    const events: unknown[] = [];
    for await (const ev of resource.streamSSE("/events")) {
      events.push(ev);
    }
    expect(events).toEqual([{ ok: true }]);
    // Last recorded request should be GET (from the stream call)
    const streamReq = mock.requests.find((r) => r.path === "/events");
    expect(streamReq?.method).toBe("GET");
  });

  it("_streamSSE skips malformed chunks", async () => {
    setup({
      "POST /stream": {
        chunks: ['{"valid":true}', "not-json", '{"also":true}'],
      },
    });

    const events: unknown[] = [];
    for await (const ev of resource.streamSSE("/stream", {})) {
      events.push(ev);
    }
    // Malformed "not-json" should be silently skipped
    expect(events).toEqual([{ valid: true }, { also: true }]);
  });

  it("_streamSSE with empty chunks yields nothing", async () => {
    setup({
      "GET /empty": { chunks: [] },
    });

    const events: unknown[] = [];
    for await (const ev of resource.streamSSE("/empty")) {
      events.push(ev);
    }
    expect(events).toEqual([]);
  });
});
