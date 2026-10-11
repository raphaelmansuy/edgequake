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
  it.each([false, true])("removes caller listeners after request failure=%s", async (fail) => {
    const controller = new AbortController();
    const add = vi.spyOn(controller.signal, "addEventListener");
    const remove = vi.spyOn(controller.signal, "removeEventListener");
    const fetchFn = vi.fn(async () => { if (fail) throw new TypeError("offline"); return response(); }) as unknown as typeof fetch;
    const pending = transport(fetchFn).request({ method: "GET", path: "/health", signal: controller.signal });
    if (fail) await expect(pending).rejects.toThrow("offline");
    else await expect(pending).resolves.toEqual({ ok: true });
    expect(add).toHaveBeenCalledTimes(1);
    expect(remove).toHaveBeenCalledWith("abort", add.mock.calls[0][1]);
  });
});
