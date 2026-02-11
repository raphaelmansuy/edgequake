/**
 * Basic Usage — EdgeQuake TypeScript SDK
 *
 * WHY: Demonstrates the simplest possible setup — create a client,
 * check health, and run a basic query. Start here if you're new.
 *
 * Usage:
 *   npx tsx examples/basic_usage.ts
 */
import { EdgeQuake } from "@edgequake/sdk";

async function main() {
  // WHY: baseUrl points to your EdgeQuake backend; apiKey authenticates.
  const client = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
  });

  // 1. Health check — verify the API is reachable
  const health = await client.health();
  console.log("Health:", health);

  // 2. Upload a simple text document
  const doc = await client.documents.upload({
    content:
      "EdgeQuake is a graph-based RAG framework written in Rust. " +
      "It uses knowledge graphs to enhance retrieval-augmented generation.",
    title: "EdgeQuake Overview",
  });
  console.log(`Uploaded document: ${doc.document_id}`);

  // 3. Query the knowledge base
  const result = await client.query.execute({
    query: "What is EdgeQuake?",
    mode: "hybrid",
  });
  console.log("Answer:", result.answer);

  // 4. Explore the graph
  const graph = await client.graph.get();
  console.log("Graph stats:", graph);
}

main().catch(console.error);
