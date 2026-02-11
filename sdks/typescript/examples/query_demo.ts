/**
 * Query Demo — EdgeQuake TypeScript SDK
 *
 * WHY: Queries are how you retrieve knowledge from EdgeQuake.
 * This example demonstrates simple, hybrid, and parametric queries.
 *
 * Usage:
 *   npx tsx examples/query_demo.ts
 */
import { EdgeQuake } from "@edgequake/sdk";

async function main() {
  const client = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
  });

  // ── 1. Simple query ───────────────────────────────────────

  // WHY: Default mode uses the backend's configured retrieval strategy.
  const simple = await client.query.execute({
    query: "What is retrieval-augmented generation?",
  });
  console.log("Simple query answer:", simple.answer);

  // ── 2. Hybrid mode query ──────────────────────────────────

  // WHY: Hybrid mode combines local (entity-centric) and global
  // (community-level) retrieval for comprehensive answers.
  const hybrid = await client.query.execute({
    query: "How do knowledge graphs improve RAG?",
    mode: "hybrid",
    top_k: 10,
  });
  console.log("\nHybrid query answer:", hybrid.answer);

  // ── 3. Chat completion (OpenAI-compatible) ────────────────

  // WHY: Chat endpoint lets you use EdgeQuake as a drop-in replacement
  // for OpenAI's chat API, with RAG context automatically injected.
  const chat = await client.chat.completions({
    model: "edgequake",
    messages: [
      {
        role: "system",
        content: "You are a helpful assistant powered by EdgeQuake.",
      },
      { role: "user", content: "What entities are in the knowledge graph?" },
    ],
  });
  console.log("\nChat response:", chat.choices?.[0]?.message?.content);
}

main().catch(console.error);
