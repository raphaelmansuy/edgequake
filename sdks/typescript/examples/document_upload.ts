/**
 * Document Upload — EdgeQuake TypeScript SDK
 *
 * WHY: Documents are the primary input to EdgeQuake. This example shows
 * text upload, PDF upload, batch upload, and async status tracking.
 *
 * Usage:
 *   npx tsx examples/document_upload.ts
 */
import { EdgeQuake } from "@edgequake/sdk";
import { readFileSync } from "node:fs";

async function main() {
  const client = new EdgeQuake({
    baseUrl: process.env.EDGEQUAKE_URL ?? "http://localhost:8080",
    apiKey: process.env.EDGEQUAKE_API_KEY ?? "demo-key",
  });

  // ── 1. Upload plain text ──────────────────────────────────

  const textDoc = await client.documents.upload({
    content:
      "Knowledge graphs represent information as nodes and edges, " +
      "enabling structured reasoning over unstructured data.",
    title: "Knowledge Graphs Introduction",
    metadata: { category: "research", author: "EdgeQuake Team" },
  });
  console.log(`Text document uploaded: ${textDoc.document_id}`);

  // ── 2. Upload a PDF file ──────────────────────────────────

  // WHY: PDF upload uses multipart/form-data under the hood.
  // The SDK handles Blob/Buffer conversion automatically.
  try {
    const pdfBuffer = readFileSync("./sample.pdf");
    const pdfDoc = await client.documents.pdf.upload(pdfBuffer, {
      title: "Sample PDF",
    });
    console.log(`PDF uploaded: ${pdfDoc.document_id}`);
  } catch {
    console.log("(Skipping PDF — no sample.pdf found in current directory)");
  }

  // ── 3. Track processing status ────────────────────────────

  // WHY: Document processing (chunking, entity extraction) is async.
  // Poll the track endpoint to monitor progress.
  if (textDoc.track_id) {
    let attempts = 0;
    while (attempts < 30) {
      const status = await client.documents.getTrackStatus(textDoc.track_id);
      console.log(`Processing: ${status.status} — ${status.message ?? ""}`);
      if (status.status === "completed" || status.status === "failed") break;
      await new Promise((r) => setTimeout(r, 2000));
      attempts++;
    }
  }

  // ── 4. List all documents (paginated) ─────────────────────

  // WHY: Paginator is an AsyncIterable — use for-await to consume.
  console.log("\nAll documents:");
  for await (const doc of client.documents.list()) {
    console.log(`  ${doc.id}: ${doc.title} (${doc.status})`);
  }

  // ── 5. Get document details ───────────────────────────────

  const detail = await client.documents.get(textDoc.document_id);
  console.log(`\nDocument detail: ${detail.title}`);
  console.log(`  Chunks: ${detail.chunk_count}`);
  console.log(`  Status: ${detail.status}`);

  // ── 6. Delete a document ──────────────────────────────────

  await client.documents.delete(textDoc.document_id);
  console.log(`\nDeleted document ${textDoc.document_id}`);
}

main().catch(console.error);
