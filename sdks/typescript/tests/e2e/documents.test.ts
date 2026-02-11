/**
 * E2E Tests: Document lifecycle
 *
 * Tests document upload, status tracking, listing, and deletion
 * against a live EdgeQuake backend.
 *
 * Run: EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/
 */

import { describe, it, expect, beforeAll, afterAll } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import {
  E2E_ENABLED,
  createE2EClient,
  waitFor,
  testId,
  sleep,
} from "./helpers.js";

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

  it("should upload a text document", async () => {
    const title = testId("doc-upload");
    const result = await client.documents.upload({
      content: "EdgeQuake is an advanced RAG framework implemented in Rust.",
      title,
    });

    expect(result).toBeDefined();
    expect(result.document_id).toBeTruthy();
    uploadedDocIds.push(result.document_id);
  });

  it("should get document status after upload", async () => {
    const title = testId("doc-status");
    const uploaded = await client.documents.upload({
      content: "Test document for status checking.",
      title,
    });
    uploadedDocIds.push(uploaded.document_id);

    const status = await client.documents.getStatus(uploaded.document_id);
    expect(status).toBeDefined();
    // Status should be one of: pending, processing, completed, failed
    expect(["pending", "processing", "completed", "failed"]).toContain(
      status.status,
    );
  });

  it("should list documents with pagination", async () => {
    const docs = await client.documents.list({ page: 1, page_size: 10 });
    expect(docs).toBeDefined();
    expect(Array.isArray(docs.items)).toBe(true);
    expect(typeof docs.total).toBe("number");
  });

  it("should get a specific document", async () => {
    const title = testId("doc-get");
    const uploaded = await client.documents.upload({
      content: "Document for retrieval test.",
      title,
    });
    uploadedDocIds.push(uploaded.document_id);

    // Small delay to allow processing
    await sleep(500);

    const doc = await client.documents.get(uploaded.document_id);
    expect(doc).toBeDefined();
    expect(doc.id).toBe(uploaded.document_id);
  });

  it("should delete a document", async () => {
    const title = testId("doc-delete");
    const uploaded = await client.documents.upload({
      content: "Document to be deleted.",
      title,
    });

    await client.documents.delete(uploaded.document_id);

    // Verify deletion — should throw NotFoundError
    try {
      await client.documents.get(uploaded.document_id);
      // If we get here, deletion didn't work
      expect.fail("Expected NotFoundError after deletion");
    } catch (error: any) {
      expect(error.status).toBe(404);
    }
  });
});
