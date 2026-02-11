/**
 * E2E Tests: Query engine
 *
 * Tests query execution, chat, and results against a live EdgeQuake backend.
 * WHY: These tests validate the most critical user-facing functionality.
 *
 * Run: EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { describe, it, expect, beforeAll } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import { E2E_ENABLED, createE2EClient, testId, sleep } from "./helpers.js";

const describeE2E = E2E_ENABLED ? describe : describe.skip;

describeE2E("E2E: Query Engine", () => {
  let client: EdgeQuake;

  beforeAll(async () => {
    client = createE2EClient()!;

    // WHY: Upload a test document so queries have something to match against
    try {
      await client.documents.upload({
        content: `
          Rust is a systems programming language focused on safety, concurrency, 
          and performance. It is used by Mozilla, Amazon, Microsoft, and Google.
          EdgeQuake uses Rust for its core RAG pipeline implementation.
        `,
        title: testId("query-seed"),
      });
      // Give the pipeline time to process
      await sleep(3000);
    } catch {
      // If upload fails, queries will return empty results — tests still valid
    }
  });

  it("should execute a simple query", async () => {
    const result = await client.query.execute({
      query: "What programming language is EdgeQuake written in?",
    });
    expect(result).toBeDefined();
    expect(typeof result.answer).toBe("string");
  });

  it("should execute a query with mode specification", async () => {
    const result = await client.query.execute({
      query: "What is Rust?",
      mode: "hybrid",
    });
    expect(result).toBeDefined();
    expect(result.answer).toBeTruthy();
  });

  it("should include sources in query response", async () => {
    const result = await client.query.execute({
      query: "Tell me about EdgeQuake",
    });
    expect(result).toBeDefined();
    // Sources may be empty if no relevant documents ingested
    if (result.sources) {
      expect(Array.isArray(result.sources)).toBe(true);
    }
  });

  it("should stream a query response", async () => {
    const chunks: string[] = [];
    const stream = client.query.stream({
      query: "What is RAG?",
    });

    for await (const event of stream) {
      if (event.chunk) {
        chunks.push(event.chunk);
      }
    }

    // WHY: Even with no documents, the LLM should generate some response
    expect(chunks.length).toBeGreaterThan(0);
  });
});

describeE2E("E2E: Chat", () => {
  let client: EdgeQuake;

  beforeAll(() => {
    client = createE2EClient()!;
  });

  it("should send a chat message and get a response", async () => {
    const result = await client.chat.send({
      message: "Hello, what can you help me with?",
    });
    expect(result).toBeDefined();
  });

  it("should stream a chat response", async () => {
    const chunks: string[] = [];
    const stream = client.chat.stream({
      message: "What is knowledge graph?",
    });

    for await (const event of stream) {
      if (event.chunk) {
        chunks.push(event.chunk);
      }
    }

    expect(chunks.length).toBeGreaterThan(0);
  });
});
