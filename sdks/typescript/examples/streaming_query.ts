/**
 * Streaming Query — EdgeQuake TypeScript SDK
 *
 * WHY: Streaming delivers tokens incrementally via Server-Sent Events (SSE),
 * enabling real-time UI updates without waiting for the full response.
 *
 * Usage:
 *   npx tsx examples/streaming_query.ts
 */
import { EdgeQuake } from "@edgequake/sdk";

async function main() {
  const client = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
  });

  // ── 1. Streaming query via SSE ────────────────────────────

  // WHY: `client.query.stream()` returns an AsyncIterable that yields
  // parsed JSON chunks as they arrive from the server.
  console.log("Streaming query response:");
  for await (const event of client.query.stream({
    query: "Explain how knowledge graphs enhance RAG systems",
    mode: "hybrid",
  })) {
    // WHY: Each event is a parsed object from the SSE data line.
    // The exact shape depends on the backend's streaming format.
    if (event.chunk) {
      process.stdout.write(event.chunk);
    }
  }
  console.log("\n");

  // ── 2. Streaming chat (OpenAI-compatible) ─────────────────

  // WHY: Chat streaming follows the OpenAI delta format, making it
  // compatible with existing OpenAI-based UIs and libraries.
  console.log("Streaming chat response:");
  for await (const chunk of client.chat.stream({
    model: "edgequake",
    messages: [
      { role: "user", content: "What are the benefits of graph-based RAG?" },
    ],
  })) {
    const delta = chunk.choices?.[0]?.delta?.content;
    if (delta) {
      process.stdout.write(delta);
    }
  }
  console.log("\n");

  // ── 3. Streaming with abort ───────────────────────────────

  // WHY: AbortController lets you cancel streaming mid-flight,
  // useful for "stop generating" buttons in UIs.
  const controller = new AbortController();

  // Cancel after 3 seconds
  setTimeout(() => {
    console.log("\n[Aborting stream after 3 seconds]");
    controller.abort();
  }, 3000);

  console.log("Streaming with abort:");
  try {
    for await (const event of client.query.stream({
      query: "Write a detailed essay about knowledge graphs",
      mode: "hybrid",
      signal: controller.signal,
    })) {
      if (event.chunk) process.stdout.write(event.chunk);
    }
  } catch (err) {
    if (err instanceof DOMException && err.name === "AbortError") {
      console.log("\n[Stream aborted successfully]");
    } else {
      throw err;
    }
  }
}

main().catch(console.error);
