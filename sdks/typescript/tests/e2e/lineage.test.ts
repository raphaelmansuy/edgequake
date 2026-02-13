/**
 * E2E Tests: Lineage & Metadata
 *
 * Tests document lineage, metadata retrieval, and chunk lineage
 * against a live EdgeQuake backend.
 *
 * Run: EDGEQUAKE_E2E_URL=http://localhost:8080 npm test -- tests/e2e/lineage.test.ts
 *
 * @implements F5 — Single API call retrieves complete document lineage tree
 * @implements F7 — All SDKs expose lineage retrieval methods
 */

import { beforeAll, describe, expect, it } from "vitest";
import { EdgeQuake } from "../../src/index.js";
import { E2E_ENABLED, createE2EClient } from "./helpers.js";

const describeE2E = E2E_ENABLED ? describe : describe.skip;

describeE2E("E2E: Lineage & Metadata", () => {
  let client: EdgeQuake;
  let firstDocId: string | undefined;

  beforeAll(async () => {
    client = createE2EClient()!;
    // Get the first available document for lineage tests
    const docs = await client.documents.list({ page: 1, page_size: 1 });
    firstDocId = docs.documents[0]?.id;
  });

  it("should get document lineage", async () => {
    if (!firstDocId) {
      console.log("Skipping — no documents available");
      return;
    }

    const lineage = await client.documents.getLineage(firstDocId);
    expect(lineage).toBeDefined();
    expect(lineage.document_id).toBe(firstDocId);
    expect(Array.isArray(lineage.chunks)).toBe(true);
    expect(Array.isArray(lineage.entities)).toBe(true);
    console.log(
      `Document lineage: ${lineage.chunks.length} chunks, ${lineage.entities.length} entities`
    );
  }, 15_000);

  it("should get document metadata", async () => {
    if (!firstDocId) {
      console.log("Skipping — no documents available");
      return;
    }

    const metadata = await client.documents.getMetadata(firstDocId);
    expect(metadata).toBeDefined();
    expect(typeof metadata).toBe("object");
    console.log(
      `Document metadata: ${Object.keys(metadata).length} fields`
    );
  }, 15_000);

  it("should get chunk lineage", async () => {
    if (!firstDocId) {
      console.log("Skipping — no documents available");
      return;
    }

    const chunkId = `${firstDocId}-chunk-0`;
    try {
      const lineage = await client.chunks.getLineage(chunkId);
      expect(lineage).toBeDefined();
      expect(lineage.chunk_id).toBe(chunkId);
      expect(lineage.document_id).toBe(firstDocId);
      console.log(
        `Chunk lineage: doc=${lineage.document_id} entities=${lineage.entity_count ?? 0}`
      );
    } catch (err: any) {
      // Chunk may not exist if document has different ID format
      if (err.status === 404) {
        console.log(`Chunk ${chunkId} not found — skipping`);
        return;
      }
      throw err;
    }
  }, 15_000);
});
