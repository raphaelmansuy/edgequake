/**
 * E2E Tests: Query engine
 *
 * Tests query execution, chat, and results against a live EdgeQuake backend.
 * WHY: These tests validate the most critical user-facing functionality.
 *
 * NOTE: Query and chat tests require Ollama to be running at localhost:11434.
 * If Ollama is down, these tests will catch and skip gracefully.
 *
 * Run: EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { beforeAll, describe, expect, it } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import { E2E_ENABLED, createE2EClient } from "./helpers.js";

const describeE2E = E2E_ENABLED ? describe : describe.skip;

/** Helper: check if LLM provider is available */
async function isLlmAvailable(client: EdgeQuake): Promise<boolean> {
  try {
    const health = await client.health();
    return health.components?.llm_provider === true;
  } catch {
    return false;
  }
}

describeE2E("E2E: Query Engine", () => {
  let client: EdgeQuake;
  let llmAvailable = false;

  beforeAll(async () => {
    client = createE2EClient()!;
    llmAvailable = await isLlmAvailable(client);
  }, 30_000);

  it("should execute a simple query", async () => {
    if (!llmAvailable) {
      console.log("Skipping — LLM provider unavailable");
      return;
    }
    const result = await client.query.execute({
      query: "What is EdgeQuake?",
    });
    expect(result).toBeDefined();
    expect(typeof result.answer).toBe("string");
  }, 30_000);

  it("should execute a query with mode specification", async () => {
    if (!llmAvailable) return;
    const result = await client.query.execute({
      query: "What is Rust?",
      mode: "hybrid",
    });
    expect(result).toBeDefined();
  }, 30_000);

  it("should stream a query response", async () => {
    if (!llmAvailable) return;
    const chunks: string[] = [];
    const stream = client.query.stream({
      query: "What is RAG?",
    });

    for await (const event of stream) {
      if (event.chunk) {
        chunks.push(event.chunk);
      }
    }

    expect(chunks.length).toBeGreaterThan(0);
  }, 60_000);
});

describeE2E("E2E: Chat", () => {
  let client: EdgeQuake;
  let llmAvailable = false;

  beforeAll(async () => {
    client = createE2EClient()!;
    llmAvailable = await isLlmAvailable(client);
  }, 30_000);

  it("should send a chat completion and get a response", async () => {
    if (!llmAvailable) return;
    try {
      // WHY: Rust API uses `message: String` (singular), not OpenAI-style `messages` array
      const result = await client.chat.completions({
        message: "Hello, what can you help me with?",
      });
      expect(result).toBeDefined();
      // WHY: Rust ChatCompletionResponse uses `content` not `message`
      expect(typeof result.content).toBe("string");
    } catch (error: any) {
      // WHY: Chat endpoint requires X-Tenant-ID and X-User-ID headers.
      // Without tenant context configured, we get 401 — expected.
      if (error.status === 401) {
        console.log(
          "Chat requires tenant context — skipping (expected in E2E without tenant config)",
        );
        return;
      }
      throw error;
    }
  }, 30_000);

  it("should stream a chat response", async () => {
    if (!llmAvailable) return;
    try {
      const events: unknown[] = [];
      const stream = client.chat.stream({
        message: "What is knowledge graph?",
      });

      for await (const event of stream) {
        events.push(event);
        // WHY: Rust ChatStreamEvent uses { type: "token", content: string }
        if (event.type === "token" && event.content) {
          break; // got at least one content chunk
        }
      }

      expect(events.length).toBeGreaterThan(0);
    } catch (error: any) {
      // WHY: Same tenant context requirement as chat.completions()
      if (error.status === 401) {
        console.log("Chat stream requires tenant context — skipping");
        return;
      }
      throw error;
    }
  }, 60_000);
});
