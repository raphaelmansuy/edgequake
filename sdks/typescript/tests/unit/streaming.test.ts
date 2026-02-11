/**
 * Streaming tests — SSE parser and WebSocket wrapper.
 */

import { describe, expect, it } from "vitest";
import { parseSSEStream } from "../../src/streaming/sse.js";

// Helper: create a mock Response with a ReadableStream body
function mockSSEResponse(text: string): Response {
  const encoder = new TextEncoder();
  const stream = new ReadableStream<Uint8Array>({
    start(controller) {
      controller.enqueue(encoder.encode(text));
      controller.close();
    },
  });
  return new Response(stream, {
    headers: { "Content-Type": "text/event-stream" },
  });
}

describe("parseSSEStream", () => {
  it("parses data: lines into typed events", async () => {
    const response = mockSSEResponse('data: {"id":1}\n\ndata: {"id":2}\n\n');
    const events: Array<{ id: number }> = [];
    for await (const ev of parseSSEStream<{ id: number }>(response, (raw) =>
      JSON.parse(raw),
    )) {
      events.push(ev);
    }
    expect(events).toEqual([{ id: 1 }, { id: 2 }]);
  });

  it("stops at [DONE] sentinel", async () => {
    const response = mockSSEResponse(
      'data: {"chunk":"Hello"}\n\ndata: [DONE]\n\ndata: {"chunk":"ignored"}\n\n',
    );
    const events: Array<{ chunk: string }> = [];
    for await (const ev of parseSSEStream<{ chunk: string }>(response, (raw) =>
      JSON.parse(raw),
    )) {
      events.push(ev);
    }
    expect(events).toEqual([{ chunk: "Hello" }]);
  });

  it("skips comment lines (starting with ':')", async () => {
    const response = mockSSEResponse(
      ': this is a comment\ndata: {"ok":true}\n\n',
    );
    const events: Array<{ ok: boolean }> = [];
    for await (const ev of parseSSEStream<{ ok: boolean }>(response, (raw) =>
      JSON.parse(raw),
    )) {
      events.push(ev);
    }
    expect(events).toEqual([{ ok: true }]);
  });

  it("skips when parser returns null", async () => {
    const response = mockSSEResponse('data: skip\n\ndata: {"keep":true}\n\n');
    const events: Array<{ keep: boolean }> = [];
    for await (const ev of parseSSEStream<{ keep: boolean }>(
      response,
      (raw) => {
        try {
          return JSON.parse(raw);
        } catch {
          return null;
        }
      },
    )) {
      events.push(ev);
    }
    expect(events).toEqual([{ keep: true }]);
  });

  it("throws if response body is null", async () => {
    const response = new Response(null);
    // Override body to null
    Object.defineProperty(response, "body", { value: null });
    const gen = parseSSEStream(response, (raw) => raw);
    await expect(gen.next()).rejects.toThrow("Response body is null");
  });

  it("handles empty data gracefully", async () => {
    const response = mockSSEResponse("\n\n");
    const events: string[] = [];
    for await (const ev of parseSSEStream<string>(response, (raw) => raw)) {
      events.push(ev);
    }
    expect(events).toEqual([]);
  });

  it("handles multi-chunk delivery", async () => {
    // Simulate network delivering data in multiple chunks
    const encoder = new TextEncoder();
    let enqueueCount = 0;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) {
        if (enqueueCount === 0) {
          controller.enqueue(encoder.encode('data: {"part":'));
          enqueueCount++;
        } else if (enqueueCount === 1) {
          controller.enqueue(encoder.encode("1}\n\n"));
          enqueueCount++;
        } else {
          controller.close();
        }
      },
    });
    const response = new Response(stream);
    const events: Array<{ part: number }> = [];
    for await (const ev of parseSSEStream<{ part: number }>(response, (raw) =>
      JSON.parse(raw),
    )) {
      events.push(ev);
    }
    expect(events).toEqual([{ part: 1 }]);
  });

  it("respects AbortSignal", async () => {
    const controller = new AbortController();
    // Create a stream that will deliver data slowly
    const encoder = new TextEncoder();
    let delivered = false;
    const stream = new ReadableStream<Uint8Array>({
      async pull(ctrl) {
        if (!delivered) {
          ctrl.enqueue(encoder.encode('data: {"id":1}\n\n'));
          delivered = true;
          // Abort before next read
          controller.abort();
        } else {
          // Wait a bit to simulate slow delivery
          await new Promise((r) => setTimeout(r, 50));
          ctrl.enqueue(encoder.encode('data: {"id":2}\n\n'));
          ctrl.close();
        }
      },
    });
    const response = new Response(stream);
    const events: unknown[] = [];
    try {
      for await (const ev of parseSSEStream(
        response,
        (raw) => JSON.parse(raw),
        controller.signal,
      )) {
        events.push(ev);
      }
    } catch (e: unknown) {
      expect((e as Error).name).toBe("AbortError");
    }
    // Should have at most 1 event before abort
    expect(events.length).toBeLessThanOrEqual(1);
  });
});
