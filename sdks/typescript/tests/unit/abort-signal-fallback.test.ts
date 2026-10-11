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
