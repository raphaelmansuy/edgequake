/**
 * Tests for EdgeQuakeWebSocket — exercises the async iterable WebSocket wrapper.
 *
 * WHY: websocket.ts wraps native WebSocket as AsyncIterable<WebSocketEvent>.
 * We mock the global WebSocket to test connect, message, close, error flows.
 */
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { EdgeQuakeWebSocket } from "../../src/streaming/websocket.js";

// ─── Global WebSocket Mock ─────────────────────────────────────

class MockWebSocket {
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  closed = false;

  constructor(public url: string) {
    // Store reference for test access
    MockWebSocket.lastInstance = this;
  }

  close() {
    this.closed = true;
    this.onclose?.();
  }

  /** Simulate receiving a message from the server. */
  simulateMessage(data: unknown) {
    this.onmessage?.({ data: JSON.stringify(data) });
  }

  /** Simulate a WebSocket error. */
  simulateError() {
    this.onerror?.();
  }

  static lastInstance: MockWebSocket | null = null;
}

// Store original WebSocket
const originalWebSocket = globalThis.WebSocket;

describe("EdgeQuakeWebSocket", () => {
  beforeEach(() => {
    MockWebSocket.lastInstance = null;
    (globalThis as Record<string, unknown>).WebSocket =
      MockWebSocket as unknown as typeof WebSocket;
  });

  afterEach(() => {
    (globalThis as Record<string, unknown>).WebSocket = originalWebSocket;
  });

  it("yields messages as WebSocketEvents", async () => {
    const ws = new EdgeQuakeWebSocket("ws://localhost:8080/ws/test");

    // Start iteration in background
    const events: unknown[] = [];
    const iterPromise = (async () => {
      for await (const event of ws) {
        events.push(event);
        if (events.length >= 2) {
          ws.close();
        }
      }
    })();

    // Let the event loop tick so connect() is called
    await new Promise((r) => setTimeout(r, 10));

    const mock = MockWebSocket.lastInstance!;
    expect(mock).toBeTruthy();
    expect(mock.url).toBe("ws://localhost:8080/ws/test");

    // Simulate server sending messages
    mock.simulateMessage({ type: "progress", progress: 50 });
    mock.simulateMessage({ type: "complete", progress: 100 });

    await iterPromise;

    expect(events).toHaveLength(2);
    expect(events[0]).toEqual({ type: "progress", progress: 50 });
    expect(events[1]).toEqual({ type: "complete", progress: 100 });
  });

  it("handles close event gracefully", async () => {
    const ws = new EdgeQuakeWebSocket("ws://localhost:8080/ws/close-test");

    const events: unknown[] = [];
    const iterPromise = (async () => {
      for await (const event of ws) {
        events.push(event);
      }
    })();

    await new Promise((r) => setTimeout(r, 10));

    const mock = MockWebSocket.lastInstance!;
    mock.simulateMessage({ type: "data", value: 1 });

    // Close from server side
    await new Promise((r) => setTimeout(r, 10));
    mock.close();

    await iterPromise;

    expect(events).toHaveLength(1);
    expect(events[0]).toEqual({ type: "data", value: 1 });
  });

  it("handles error event gracefully", async () => {
    const ws = new EdgeQuakeWebSocket("ws://localhost:8080/ws/error-test");

    const events: unknown[] = [];
    const iterPromise = (async () => {
      for await (const event of ws) {
        events.push(event);
      }
    })();

    await new Promise((r) => setTimeout(r, 10));

    const mock = MockWebSocket.lastInstance!;
    mock.simulateError();

    await iterPromise;

    expect(events).toHaveLength(0); // Error closes iteration
  });

  it("skips malformed JSON messages", async () => {
    const ws = new EdgeQuakeWebSocket("ws://localhost:8080/ws/malformed");

    const events: unknown[] = [];
    const iterPromise = (async () => {
      for await (const event of ws) {
        events.push(event);
        if (events.length >= 1) ws.close();
      }
    })();

    await new Promise((r) => setTimeout(r, 10));

    const mock = MockWebSocket.lastInstance!;

    // Send malformed data — raw string (not wrapped in JSON.stringify by test)
    mock.onmessage?.({ data: "not-valid-json{" });

    // Send valid data
    mock.simulateMessage({ type: "ok" });

    await iterPromise;

    expect(events).toHaveLength(1);
    expect(events[0]).toEqual({ type: "ok" });
  });

  it("close() method closes underlying WebSocket", async () => {
    const ws = new EdgeQuakeWebSocket("ws://localhost:8080/ws/manual-close");

    // Start iteration
    const iterPromise = (async () => {
      for await (const _event of ws) {
        // won't receive anything
      }
    })();

    await new Promise((r) => setTimeout(r, 10));

    const mock = MockWebSocket.lastInstance!;
    ws.close();

    await iterPromise;

    expect(mock.closed).toBe(true);
  });

  it("buffers events received before iteration pull", async () => {
    const ws = new EdgeQuakeWebSocket("ws://localhost:8080/ws/buffer");

    const events: unknown[] = [];

    // Start iteration in background
    const iterPromise = (async () => {
      for await (const event of ws) {
        events.push(event);
        if (events.length >= 3) ws.close();
      }
    })();

    await new Promise((r) => setTimeout(r, 10));

    const mock = MockWebSocket.lastInstance!;

    // Rapidly send multiple messages before iteration can process them
    mock.simulateMessage({ seq: 1 });
    mock.simulateMessage({ seq: 2 });
    mock.simulateMessage({ seq: 3 });

    await iterPromise;

    expect(events).toHaveLength(3);
    expect(events[0]).toEqual({ seq: 1 });
    expect(events[1]).toEqual({ seq: 2 });
    expect(events[2]).toEqual({ seq: 3 });
  });
});
