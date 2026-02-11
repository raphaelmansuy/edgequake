/**
 * Batch Operations — EdgeQuake TypeScript SDK
 *
 * WHY: Batch operations reduce HTTP round-trips for bulk workflows —
 * upload many documents, bulk delete conversations, paginate large lists.
 *
 * Usage:
 *   npx tsx examples/batch_operations.ts
 */
import { EdgeQuake } from "@edgequake/sdk";

async function main() {
  const client = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
  });

  // ── 1. Batch document upload ──────────────────────────────

  // WHY: Upload multiple documents sequentially with progress tracking.
  const documents = [
    { title: "Doc 1", content: "First document about knowledge graphs." },
    { title: "Doc 2", content: "Second document about entity extraction." },
    { title: "Doc 3", content: "Third document about graph traversal." },
    { title: "Doc 4", content: "Fourth document about RAG pipelines." },
    { title: "Doc 5", content: "Fifth document about vector embeddings." },
  ];

  const uploadedIds: string[] = [];
  for (const doc of documents) {
    const result = await client.documents.upload(doc);
    uploadedIds.push(result.document_id);
    console.log(`Uploaded: ${doc.title} → ${result.document_id}`);
  }

  // ── 2. Paginated document listing ─────────────────────────

  // WHY: Paginator is an AsyncIterable that automatically fetches
  // next pages. Use `break` to stop early.
  console.log("\nAll documents (paginated):");
  let count = 0;
  for await (const doc of client.documents.list({ limit: 2 })) {
    console.log(`  [${++count}] ${doc.id}: ${doc.title}`);
    if (count >= 10) break; // Safety limit for demo
  }

  // ── 3. Bulk conversation operations ───────────────────────

  // WHY: Bulk operations act on arrays of IDs in a single request,
  // much faster than individual DELETE/PATCH calls.
  const conv1 = await client.conversations.create({ title: "Convo A" });
  const conv2 = await client.conversations.create({ title: "Convo B" });
  console.log(`\nCreated conversations: ${conv1.id}, ${conv2.id}`);

  // Bulk delete
  await client.conversations.bulkDelete({ ids: [conv1.id, conv2.id] });
  console.log("Bulk deleted 2 conversations");

  // ── 4. Delete all documents ───────────────────────────────

  // WHY: deleteAll removes all documents in the current workspace.
  // Use with caution — this is irreversible.
  await client.documents.deleteAll();
  console.log("\nDeleted all documents");

  // ── 5. Reprocess failed documents ─────────────────────────

  // WHY: If document processing fails (e.g., LLM timeout), you can
  // trigger a reprocessing of all failed documents.
  try {
    const reprocessed = await client.documents.reprocessFailed();
    console.log("Reprocessed failed documents:", reprocessed);
  } catch {
    console.log("(No failed documents to reprocess)");
  }

  // ── 6. Pipeline cost estimation ───────────────────────────

  // WHY: Before uploading large batches, estimate the LLM cost.
  try {
    const estimate = await client.pipeline.estimateCost({
      document_count: 100,
      avg_chunk_count: 20,
    });
    console.log("\nCost estimate for 100 documents:", estimate);
  } catch {
    console.log("(Cost estimation not available)");
  }
}

main().catch(console.error);
