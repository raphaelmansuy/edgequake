import { QueryClient } from "@tanstack/react-query";
import { afterEach, describe, expect, it } from "vitest";

import type { ReprocessFailedResponse } from "@/lib/api/edgequake";
import type { Document } from "@/types";

import {
  applyReprocessSuccessToCache,
  beginProvisionalReprocess,
  clearDeferredUnpinTimersForTests,
  clearReprocessPinsForTests,
  documentIdsWithQueuingSession,
  filterRunsExcludingQueuingSession,
  formatReprocessSkipReasons,
  isPollableReprocessProgressTrackId,
  isProvisionalReprocessTrackId,
  isReprocessBatchTrackId,
  isReprocessPinned,
  patchDocumentsReprocessOptimistic,
  pinDocumentShell,
  protectPinnedDocumentsInQueryData,
  provisionalReprocessTrackId,
  resolveReprocessPanelTrackId,
  resolveReprocessProgressTrackId,
  shouldShowReprocessQueuingPanel,
  unpinReprocessDocuments,
} from "../reprocess-cache";

function makeDoc(id: string, overrides: Partial<Document> = {}): Document {
  return {
    id,
    title: id,
    status: "completed",
    track_id: "old-completed-track",
    source_type: "pdf",
    ...overrides,
  } as Document;
}

function seedDocuments(queryClient: QueryClient, docs: Document[]) {
  queryClient.setQueryData(["documents", "ws-1"], { items: docs });
}

afterEach(() => {
  clearReprocessPinsForTests();
});

describe("resolveReprocessProgressTrackId", () => {
  it("prefers per-doc task_id over batch reprocess_*", () => {
    const response: ReprocessFailedResponse = {
      track_id: "reprocess_20260716_120000_abcd",
      failed_found: 1,
      requeued: 1,
      document_ids: ["doc-1"],
      task_id: "pdf_processing-aaa",
      document_task_ids: [
        { document_id: "doc-1", task_id: "pdf_processing-bbb" },
      ],
    };
    expect(resolveReprocessProgressTrackId(response, "doc-1")).toBe(
      "pdf_processing-bbb",
    );
  });

  it("falls back to top-level task_id then batch track_id", () => {
    expect(
      resolveReprocessProgressTrackId(
        {
          track_id: "reprocess_batch",
          failed_found: 1,
          requeued: 1,
          document_ids: ["doc-1"],
          task_id: "pdf_processing-only",
        },
        "doc-1",
      ),
    ).toBe("pdf_processing-only");

    expect(
      resolveReprocessProgressTrackId(
        {
          track_id: "reprocess_batch",
          failed_found: 1,
          requeued: 1,
          document_ids: ["doc-1"],
        },
        "doc-1",
      ),
    ).toBe("reprocess_batch");
  });
});

describe("panel track classification (Queuing vs poll)", () => {
  it("classifies provisional, batch, and live keys", () => {
    expect(isProvisionalReprocessTrackId("reprocess_pending_doc-1")).toBe(true);
    expect(isReprocessBatchTrackId("reprocess_pending_doc-1")).toBe(false);
    expect(isPollableReprocessProgressTrackId("reprocess_pending_doc-1")).toBe(
      false,
    );
    expect(shouldShowReprocessQueuingPanel("reprocess_pending_doc-1")).toBe(
      true,
    );

    expect(isReprocessBatchTrackId("reprocess_20260716_120000_abcd")).toBe(
      true,
    );
    expect(
      isPollableReprocessProgressTrackId("reprocess_20260716_120000_abcd"),
    ).toBe(false);
    expect(
      shouldShowReprocessQueuingPanel("reprocess_20260716_120000_abcd"),
    ).toBe(true);

    expect(isPollableReprocessProgressTrackId("pdf_processing-live")).toBe(
      true,
    );
    expect(shouldShowReprocessQueuingPanel("pdf_processing-live")).toBe(false);
  });

  it("resolveReprocessPanelTrackId keeps Queuing during early-admit batch poll", () => {
    expect(
      resolveReprocessPanelTrackId(
        "reprocess_pending_doc-1",
        "reprocess_20260716_batch",
      ),
    ).toBe("reprocess_pending_doc-1");

    // Even if server already has a live key, keep provisional until entry upgrades.
    expect(
      resolveReprocessPanelTrackId(
        "reprocess_pending_doc-1",
        "pdf_processing-live",
      ),
    ).toBe("reprocess_pending_doc-1");

    expect(
      resolveReprocessPanelTrackId(
        "pdf_processing-entry",
        "pdf_processing-server",
      ),
    ).toBe("pdf_processing-server");

    expect(
      resolveReprocessPanelTrackId(
        "pdf_processing-entry",
        "reprocess_20260716_batch",
      ),
    ).toBe("pdf_processing-entry");
  });
});

describe("provisional + pin (immediate feedback)", () => {
  it("beginProvisionalReprocess seeds pending track ids and pins", () => {
    const queryClient = new QueryClient();
    seedDocuments(queryClient, [makeDoc("doc-1"), makeDoc("doc-2")]);

    const provisional = beginProvisionalReprocess(queryClient, ["doc-1"]);
    expect(provisional.get("doc-1")).toBe(provisionalReprocessTrackId("doc-1"));
    expect(isProvisionalReprocessTrackId(provisional.get("doc-1"))).toBe(true);
    expect(isReprocessPinned("doc-1")).toBe(true);

    const cached = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    const doc = cached?.items.find((d) => d.id === "doc-1");
    expect(doc?.status).toBe("processing");
    expect(doc?.current_stage).toBe("cleaning");
    expect(doc?.stage_message).toMatch(/Removing prior knowledge graph/i);
    expect(doc?.track_id).toBe("reprocess_pending_doc-1");
    expect(cached?.items.find((d) => d.id === "doc-2")?.status).toBe(
      "completed",
    );
  });

  it("protectPinnedDocumentsInQueryData blocks poll overwrite to completed", () => {
    const queryClient = new QueryClient();
    seedDocuments(queryClient, [makeDoc("doc-1")]);
    beginProvisionalReprocess(queryClient, "doc-1");

    const polled = protectPinnedDocumentsInQueryData({
      items: [makeDoc("doc-1", { status: "completed", track_id: "stale" })],
    });
    expect(polled.items?.[0]?.status).toBe("processing");
    expect(polled.items?.[0]?.current_stage).toBe("cleaning");
    expect(polled.items?.[0]?.stage_message).toMatch(
      /Removing prior knowledge graph/i,
    );
    expect(polled.items?.[0]?.track_id).toBe("reprocess_pending_doc-1");
  });

  it("success binds live track and keeps pin against stale Completed poll", () => {
    const queryClient = new QueryClient();
    seedDocuments(queryClient, [makeDoc("doc-1")]);
    beginProvisionalReprocess(queryClient, "doc-1");

    const progressId = applyReprocessSuccessToCache(queryClient, "doc-1", {
      track_id: "reprocess_batch",
      failed_found: 1,
      requeued: 1,
      document_ids: ["doc-1"],
      task_id: "pdf_processing-live",
    });
    expect(progressId).toBe("pdf_processing-live");
    // Deferred unpin: still pinned so stale Completed cannot flash.
    expect(isReprocessPinned("doc-1")).toBe(true);

    const cached = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(cached?.items[0]?.track_id).toBe("pdf_processing-live");

    const stalePoll = protectPinnedDocumentsInQueryData({
      items: [makeDoc("doc-1", { status: "completed", track_id: "stale" })],
    });
    expect(stalePoll.items?.[0]?.status).toBe("processing");
    expect(stalePoll.items?.[0]?.track_id).toBe("pdf_processing-live");

    clearDeferredUnpinTimersForTests();
    unpinReprocessDocuments("doc-1");
    expect(isReprocessPinned("doc-1")).toBe(false);
  });

  it("honest server processing with live track releases pin early", () => {
    const queryClient = new QueryClient();
    seedDocuments(queryClient, [makeDoc("doc-1")]);
    beginProvisionalReprocess(queryClient, "doc-1");
    applyReprocessSuccessToCache(queryClient, "doc-1", {
      track_id: "reprocess_batch",
      failed_found: 1,
      requeued: 1,
      document_ids: ["doc-1"],
      task_id: "pdf_processing-live",
    });
    expect(isReprocessPinned("doc-1")).toBe(true);

    protectPinnedDocumentsInQueryData({
      items: [
        makeDoc("doc-1", {
          status: "processing",
          current_stage: "converting",
          track_id: "pdf_processing-live",
        }),
      ],
    });
    expect(isReprocessPinned("doc-1")).toBe(false);
  });

  it("unpin allows rollback after skip", () => {
    beginProvisionalReprocess(new QueryClient(), "doc-1");
    expect(isReprocessPinned("doc-1")).toBe(true);
    unpinReprocessDocuments("doc-1");
    expect(isReprocessPinned("doc-1")).toBe(false);
  });

  it("session dismiss unpin releases optimistic processing immediately", () => {
    const queryClient = new QueryClient();
    seedDocuments(queryClient, [makeDoc("doc-1", { status: "completed" })]);
    beginProvisionalReprocess(queryClient, "doc-1");
    expect(isReprocessPinned("doc-1")).toBe(true);

    // Contract: DocumentManager dismissSessionPanel calls unpin before remove.
    unpinReprocessDocuments("doc-1");
    expect(isReprocessPinned("doc-1")).toBe(false);

    const afterDismiss = protectPinnedDocumentsInQueryData({
      items: [makeDoc("doc-1", { status: "completed", track_id: null })],
    });
    expect(afterDismiss.items?.[0]?.status).toBe("completed");
  });

  it("filterRunsExcludingQueuingSession hides ActiveRuns during Queuing", () => {
    const queuing = documentIdsWithQueuingSession([
      { documentId: "doc-1", trackId: "reprocess_pending_doc-1" },
      { documentId: "doc-2", trackId: "pdf_processing-live" },
    ]);
    expect(queuing.has("doc-1")).toBe(true);
    expect(queuing.has("doc-2")).toBe(false);

    const filtered = filterRunsExcludingQueuingSession(
      [
        { documentId: "doc-1", name: "a" },
        { documentId: "doc-2", name: "b" },
      ],
      queuing,
    );
    expect(filtered.map((r) => r.documentId)).toEqual(["doc-2"]);
  });

  it("keeps ActiveRuns during cleaning admission (dual-UI)", () => {
    const stages = new Map<string, string>([["doc-1", "cleaning"]]);
    const queuing = documentIdsWithQueuingSession(
      [{ documentId: "doc-1", trackId: "reprocess_pending_doc-1" }],
      stages,
    );
    expect(queuing.has("doc-1")).toBe(false);

    const filtered = filterRunsExcludingQueuingSession(
      [{ documentId: "doc-1", stage: "cleaning" }],
      new Set(["doc-1"]),
    );
    expect(filtered.map((r) => r.documentId)).toEqual(["doc-1"]);
  });

  it("pinDocumentShell re-injects row dropped by poll", () => {
    pinDocumentShell(
      makeDoc("upload-1", {
        status: "processing",
        track_id: "pdf_processing-up",
      }),
    );
    const protectedData = protectPinnedDocumentsInQueryData({
      items: [makeDoc("other", { status: "completed" })],
    });
    expect(protectedData.items?.some((d) => d.id === "upload-1")).toBe(true);
    expect(
      protectedData.items?.find((d) => d.id === "upload-1")?.track_id,
    ).toBe("pdf_processing-up");
  });

  it("does not reinject pin when list has staging: alias or same track_id", () => {
    const bareId = "doc-md-1";
    const track = "insert-md-1";
    pinDocumentShell(
      makeDoc(bareId, {
        status: "pending",
        current_stage: "uploading",
        stage_message: "Queued for processing…",
        track_id: track,
        file_name: "wiki.md",
        source_type: "markdown",
      }),
    );

    // Legacy list id drift: staging:{uuid} with same track.
    const protectedData = protectPinnedDocumentsInQueryData({
      items: [
        makeDoc(`staging:${bareId}`, {
          status: "processing",
          current_stage: "extracting",
          stage_message: "Extracting…",
          track_id: track,
          file_name: "wiki.md",
          source_type: "markdown",
        }),
      ],
    });

    const ids = (protectedData.items ?? []).map((d) => d.id);
    expect(ids).toEqual([`staging:${bareId}`]);
    expect(ids).not.toContain(bareId);
    expect(isReprocessPinned(bareId)).toBe(false);
  });
});

describe("applyReprocessSuccessToCache", () => {
  it("binds task_id and processing/queued; returns progress key", () => {
    const queryClient = new QueryClient();
    seedDocuments(queryClient, [makeDoc("doc-1")]);

    const response: ReprocessFailedResponse = {
      track_id: "reprocess_batch",
      failed_found: 1,
      requeued: 1,
      document_ids: ["doc-1"],
      task_id: "pdf_processing-live",
    };

    const progressId = applyReprocessSuccessToCache(
      queryClient,
      "doc-1",
      response,
    );
    expect(progressId).toBe("pdf_processing-live");

    const cached = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    const doc = cached?.items.find((d) => d.id === "doc-1");
    expect(doc?.status).toBe("processing");
    expect(doc?.current_stage).toBe("queued");
    expect(doc?.stage_message).toMatch(/Waiting for a free worker/i);
    expect(doc?.track_id).toBe("pdf_processing-live");
  });

  it("returns null when requeued is 0 and does not seed a progress key", () => {
    const queryClient = new QueryClient();
    seedDocuments(queryClient, [makeDoc("doc-1")]);

    const progressId = applyReprocessSuccessToCache(queryClient, "doc-1", {
      track_id: "reprocess_batch",
      failed_found: 1,
      requeued: 0,
      skipped: 1,
      skip_reasons: { already_processing: 1 },
      document_ids: [],
      task_id: "pdf_processing-should-not-bind",
    });

    expect(progressId).toBeNull();
    const cached = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(cached?.items[0]?.track_id).toBe("old-completed-track");
  });
});

describe("patchDocumentsReprocessOptimistic", () => {
  it("marks selected docs processing/cleaning without requiring track_id", () => {
    const queryClient = new QueryClient();
    seedDocuments(queryClient, [makeDoc("a"), makeDoc("b")]);
    patchDocumentsReprocessOptimistic(queryClient, ["a"]);
    const cached = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(cached?.items.find((d) => d.id === "a")?.status).toBe("processing");
    expect(cached?.items.find((d) => d.id === "a")?.current_stage).toBe(
      "cleaning",
    );
    expect(cached?.items.find((d) => d.id === "b")?.status).toBe("completed");
  });
});

describe("formatReprocessSkipReasons", () => {
  it("formats reason counts", () => {
    expect(
      formatReprocessSkipReasons({ already_processing: 1, no_content: 2 }),
    ).toBe("already_processing (1), no_content (2)");
    expect(formatReprocessSkipReasons(undefined)).toBe("");
  });
});
