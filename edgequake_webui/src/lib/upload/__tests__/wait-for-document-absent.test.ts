/**
 * SPEC-086 ops: Replace must wait until the old row is gone from RQ cache.
 */
import { describe, expect, it, vi } from "bun:test";
import { QueryClient } from "@tanstack/react-query";
import { waitForDocumentAbsent } from "@/lib/upload/wait-for-document-absent";

describe("waitForDocumentAbsent", () => {
  it("resolves immediately when document is already absent", async () => {
    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    qc.setQueryData(["documents", "list"], { items: [] });
    await waitForDocumentAbsent(qc, "doc-gone", {
      timeoutMs: 1_000,
      intervalMs: 10,
    });
  });

  it("waits until the document leaves the documents cache", async () => {
    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    qc.setQueryData(["documents", "list"], {
      items: [{ id: "doc-stay", file_name: "a.md" }],
    });

    let invalidations = 0;
    const invalidate = qc.invalidateQueries.bind(qc);
    vi.spyOn(qc, "invalidateQueries").mockImplementation(async (filters) => {
      invalidations += 1;
      if (invalidations >= 2) {
        qc.setQueryData(["documents", "list"], { items: [] });
      }
      return invalidate(filters);
    });

    await waitForDocumentAbsent(qc, "doc-stay", {
      timeoutMs: 5_000,
      intervalMs: 10,
    });
    expect(invalidations).toBeGreaterThanOrEqual(2);
  });

  it("fails closed on timeout while document remains", async () => {
    const qc = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    qc.setQueryData(["documents", "list"], {
      items: [{ id: "doc-stuck", file_name: "b.md" }],
    });
    vi.spyOn(qc, "invalidateQueries").mockResolvedValue(undefined);

    await expect(
      waitForDocumentAbsent(qc, "doc-stuck", {
        timeoutMs: 80,
        intervalMs: 20,
      }),
    ).rejects.toThrow(/Timed out waiting for document/);
  });
});
