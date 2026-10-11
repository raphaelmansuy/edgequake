import { describe, expect, it } from "vitest";
import { parseSSEStream } from "../../src/streaming/sse.js";
import { FetchTransport } from "../../src/transport/fetch.js";

function consume(response: Response, transport: boolean, signal?: AbortSignal): AsyncIterable<unknown> {
  if (!transport) return parseSSEStream(response, JSON.parse, signal);
  return new FetchTransport({ baseUrl: "https://example.invalid", headers: {}, timeout: 0,
    maxRetries: 0, retryDelay: 0, retryStatusCodes: [], fetchFn: async () => response
  }).stream({ method: "GET", path: "/query", signal });
}

for (const transport of [false, true]) {
  describe(`SSE cancellation (${transport ? "transport" : "typed"})`, () => {
    it("cancels an open response when the consumer breaks", async () => {
      let cancellations = 0;
      const response = new Response(new ReadableStream({
        start(controller) { controller.enqueue(new TextEncoder().encode('data: {"ok":true}\n\n')); },
        cancel() { cancellations++; },
      }));
      for await (const _event of consume(response, transport)) break;
      expect(cancellations).toBe(1);
      expect(response.body!.locked).toBe(false);
    });
    it("cancels the response at the DONE sentinel", async () => {
      let cancellations = 0;
      const response = new Response(new ReadableStream({
        start(controller) { controller.enqueue(new TextEncoder().encode('data: [DONE]\n\n')); },
        cancel() { cancellations++; },
      }));
      for await (const _event of consume(response, transport)) throw new Error("unexpected event");
      expect(cancellations).toBe(1);
    });
    it("aborts a pending read and cancels the underlying source", async () => {
      let cancellations = 0;
      let controller!: ReadableStreamDefaultController<Uint8Array>;
      const response = new Response(new ReadableStream<Uint8Array>({
        start(value) { controller = value; },
        cancel() { cancellations++; },
      }));
      const abort = new AbortController();
      const iterator = consume(response, transport, abort.signal)[Symbol.asyncIterator]();
      const pending = iterator.next().then(() => "completed", (error: Error) => error.name);
      await Promise.resolve();
      abort.abort();
      const result = await Promise.race([pending, new Promise<string>((resolve) => setTimeout(() => resolve("timed out"), 100))]);
      if (result === "timed out") controller.close(); // Bound cleanup on the unpatched implementation.
      await pending;
      expect(result).toBe("AbortError");
      expect(cancellations).toBe(1);
      expect(response.body!.locked).toBe(false);
    });
    it("preserves successful EOF without cancelling an already closed source", async () => {
      let cancellations = 0;
      const response = new Response(new ReadableStream({
        start(controller) { controller.close(); },
        cancel() { cancellations++; },
      }));
      for await (const _event of consume(response, transport)) throw new Error("unexpected event");
      expect(cancellations).toBe(0);
      expect(response.body!.locked).toBe(false);
    });
  });
}

it("preserves a parser error when the source's cancellation rejects", async () => {
  const response = new Response(new ReadableStream({
    start(controller) { controller.enqueue(new TextEncoder().encode("data: invalid-json\n\n")); },
    cancel() { return Promise.reject(new Error("cleanup failure")); },
  }));
  await expect(parseSSEStream(response, JSON.parse).next()).rejects.toBeInstanceOf(SyntaxError);
  expect(response.body!.locked).toBe(false);
});

it.each([false, true])("preserves a failed response's original error (%s)", async (transport) => {
  const response = new Response(new ReadableStream({
    start(controller) { controller.error(new Error("source failure")); },
  }));
  const iterator = consume(response, transport)[Symbol.asyncIterator]();
  await expect(iterator.next()).rejects.toThrow("source failure");
  expect(response.body!.locked).toBe(false);
});
