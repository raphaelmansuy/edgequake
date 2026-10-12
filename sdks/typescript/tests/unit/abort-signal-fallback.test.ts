import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { FetchTransport } from "../../src/transport/fetch.js";

const anyDescriptor = Object.getOwnPropertyDescriptor(AbortSignal, "any")!;
function transport(fetchFn: typeof fetch) {
  return new FetchTransport({ baseUrl: "http://localhost:8080", headers: {}, timeout: 0,
    maxRetries: 0, retryDelay: 0, retryStatusCodes: [], fetchFn });
}
function response() {
  return new Response('{"ok":true}', { headers: { "Content-Type": "application/json" } });
}
describe("AbortSignal composition fallback", () => {
  beforeEach(() => { Reflect.deleteProperty(AbortSignal, "any"); });
  afterEach(() => { Object.defineProperty(AbortSignal, "any", anyDescriptor); vi.restoreAllMocks(); });
  it("propagates an already aborted user signal and its reason", async () => {
    const controller = new AbortController();
    const reason = new DOMException("caller cancellation", "AbortError");
    controller.abort(reason);
    const fetchFn = vi.fn(async (_url, init) => {
      const signal = init!.signal!;
      if (signal.aborted) throw signal.reason;
      return response();
    }) as unknown as typeof fetch;
    await expect(transport(fetchFn).request({ method: "GET", path: "/health", signal: controller.signal })).rejects.toBe(reason);
  });
  it("preserves the reason when the caller aborts a pending request", async () => {
    const controller = new AbortController();
    const reason = new DOMException("later cancellation", "AbortError");
    const fetchFn = vi.fn((_url, init) => new Promise<Response>((_resolve, reject) => {
      init!.signal!.addEventListener("abort", () => reject(init!.signal!.reason), { once: true });
    })) as unknown as typeof fetch;
    const pending = transport(fetchFn).request({ method: "GET", path: "/health", signal: controller.signal });
    controller.abort(reason);
    await expect(pending).rejects.toBe(reason);
  });
  it("keeps caller cancellation active while consuming the response body", async () => {
    const controller = new AbortController();
    const reason = new DOMException("body cancellation", "AbortError");
    let headersReturned!: () => void;
    const headersReady = new Promise<void>((resolve) => { headersReturned = resolve; });
    const fetchFn = vi.fn(async (_url, init) => {
      const body = new ReadableStream<Uint8Array>({
        start(stream) {
          init!.signal!.addEventListener("abort", () => stream.error(init!.signal!.reason), { once: true });
        },
      });
      headersReturned();
      return new Response(body, { headers: { "Content-Type": "application/json" } });
    }) as unknown as typeof fetch;
    const pending = transport(fetchFn).request({ method: "GET", path: "/health", signal: controller.signal });
    await headersReady;
    await Promise.resolve();
    controller.abort(reason);
    await expect(pending).rejects.toBe(reason);
  });
  it.each(["success", "body failure", "fetch failure", "no content"])(
    "releases fallback listeners after %s with a reusable caller signal", async (outcome) => {
      const controller = new AbortController();
      const added = vi.spyOn(controller.signal, "addEventListener");
      const removed = vi.spyOn(controller.signal, "removeEventListener");
      const fetchFn = vi.fn(async () => {
        if (outcome === "fetch failure") throw new Error("fetch failed");
        if (outcome === "body failure") return new Response("invalid json", { headers: { "Content-Type": "application/json" } });
        if (outcome === "no content") return new Response(null, { status: 204 });
        return response();
      }) as unknown as typeof fetch;
      for (let attempt = 0; attempt < 3; attempt++) {
        const pending = transport(fetchFn).request({ method: "GET", path: "/health", signal: controller.signal });
        if (outcome.includes("failure")) await expect(pending).rejects.toThrow();
        else await pending;
      }
      expect(added).toHaveBeenCalledTimes(3);
      expect(removed).toHaveBeenCalledTimes(3);
      for (const [, listener] of added.mock.calls) {
        expect(removed).toHaveBeenCalledWith("abort", listener);
      }
    },
  );
  it("releases fallback listeners when a stream consumer stops early", async () => {
    const controller = new AbortController();
    const removed = vi.spyOn(controller.signal, "removeEventListener");
    const fetchFn = vi.fn(async () => new Response("data: first\n\ndata: second\n\n")) as unknown as typeof fetch;
    for await (const data of transport(fetchFn).stream({ method: "GET", path: "/stream", signal: controller.signal })) {
      expect(data).toBe("first");
      break;
    }
    expect(removed).toHaveBeenCalledOnce();
  });
  it("releases listeners from every middleware attempt after body consumption", async () => {
    const controller = new AbortController();
    const added = vi.spyOn(controller.signal, "addEventListener");
    const removed = vi.spyOn(controller.signal, "removeEventListener");
    const client = new FetchTransport({ baseUrl: "http://localhost:8080", headers: {}, timeout: 0,
      maxRetries: 0, retryDelay: 0, retryStatusCodes: [], fetchFn: async () => response() }, [
      async (options, next) => { await next(options); return next(options); },
    ]);
    await client.request({ method: "GET", path: "/health", signal: controller.signal });
    expect(added).toHaveBeenCalledTimes(2);
    expect(removed).toHaveBeenCalledTimes(2);
    for (const [, listener] of added.mock.calls) {
      expect(removed).toHaveBeenCalledWith("abort", listener);
    }
  });
  it("preserves native signal composition when available", async () => {
    Object.defineProperty(AbortSignal, "any", anyDescriptor);
    const controller = new AbortController();
    const reason = new DOMException("native cancellation", "AbortError");
    controller.abort(reason);
    const fetchFn = vi.fn(async (_url, init) => {
      if (init!.signal!.aborted) throw init!.signal!.reason;
      return response();
    }) as unknown as typeof fetch;
    await expect(transport(fetchFn).request({ method: "GET", path: "/health", signal: controller.signal })).rejects.toBe(reason);
  });
});
