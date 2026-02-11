/**
 * E2E Tests: Document lifecycle
 *
 * Tests document upload, status tracking, listing, and deletion
 * against a live EdgeQuake backend.
 *
 * Run: EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import { E2E_ENABLED, createE2EClient, sleep, testId } from "./helpers.js";

const describeE2E = E2E_ENABLED ? describe : describe.skip;

describeE2E("E2E: Document Lifecycle", () => {
  let client: EdgeQuake;
  const uploadedDocIds: string[] = [];

  beforeAll(() => {
    client = createE2EClient()!;
  });

  // WHY: Clean up test documents to avoid polluting the server
  afterAll(async () => {
    for (const docId of uploadedDocIds) {
      try {
        await client.documents.delete(docId);
      } catch {
        // Ignore cleanup errors — may already be deleted
      }
    }
  });

  it("should list documents with pagination", async () => {
    // WHY: list() now returns ListDocumentsResponse directly (matches Rust API)
    const res = await client.documents.list({ page: 1, page_size: 10 });
    expect(res).toBeDefined();
    expect(Array.isArray(res.documents)).toBe(true);
    expect(typeof res.total).toBe("number");
    expect(typeof res.has_more).toBe("boolean");
  });

  it("should upload a text document", async () => {
    const title = testId("doc-upload");
    const result = await client.documents.upload({
      content: "EdgeQuake is an advanced RAG framework implemented in Rust.",
      title,
    });

    expect(result).toBeDefined();
    expect(result.document_id).toBeTruthy();
    uploadedDocIds.push(result.document_id);
  }, 30_000); // WHY: Upload triggers pipeline processing which may take time

  it("should get a specific document after upload", async () => {
    // WHY: Use an ID from a previous test or get the latest document
    if (uploadedDocIds.length === 0) return; // Skip if upload failed
    const docId = uploadedDocIds[0];

    await sleep(1000); // Small delay for processing
    const doc = await client.documents.get(docId);
    expect(doc).toBeDefined();
  }, 15_000);

  it("should delete a document", async () => {
    const title = testId("doc-delete");
    try {
      const uploaded = await client.documents.upload({
        content: "Document to be deleted.",
        title,
      });

      // WHY: Document may still be processing — wait a bit then try delete
      // If it's still processing, the API returns 409 Conflict which is expected.
      await sleep(2000);

      try {
        await client.documents.delete(uploaded.document_id);
      } catch (deleteErr: any) {
        // 409 Conflict = still processing — not a test failure, just cleanup later
        if (deleteErr.status === 409) {
          console.log("Document still processing — skip delete verification");
          uploadedDocIds.push(uploaded.document_id);
          return;
        }
        throw deleteErr;
      }

      // Verify deletion — should throw NotFoundError
      try {
        await client.documents.get(uploaded.document_id);
        expect.fail("Expected NotFoundError after deletion");
      } catch (error: any) {
        expect(error.status).toBe(404);
      }
    } catch (error: any) {
      // WHY: If Ollama is down, upload may fail with 500 — skip gracefully
      if (error.status === 500 && error.message?.includes("LLM error")) {
        console.log("Skipping delete test — LLM unavailable");
        return;
      }
      throw error;
    }
  }, 30_000);
});
