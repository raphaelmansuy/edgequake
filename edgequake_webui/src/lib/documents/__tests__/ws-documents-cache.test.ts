import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import type { Document } from "@/types";

import {
  DOCUMENTS_INVALIDATE_DEBOUNCE_MS,
  DOCUMENTS_SAFETY_NET_INVALIDATE_MS,
  isListNoiseProgressEvent,
  patchDocumentsCacheFromProgress,
  shouldInvalidateDocumentsList,
  shouldPatchDocumentsCache,
} from "../ws-documents-cache";

function makeDoc(id: string, overrides: Partial<Document> = {}): Document {
  return {
    id,
    title: id,
    status: "processing",
    track_id: `track-${id}`,
    source_type: "pdf",
    ...overrides,
  } as Document;
}

describe("ws-documents-cache classification", () => {
  it("treats ChunkProgress as list noise", () => {
    expect(isListNoiseProgressEvent("ChunkProgress")).toBe(true);
    expect(shouldPatchDocumentsCache("ChunkProgress")).toBe(false);
    expect(shouldInvalidateDocumentsList("ChunkProgress")).toBe(false);
  });

  it("patches stage_progress without full invalidate", () => {
    expect(shouldPatchDocumentsCache("stage_progress")).toBe(true);
    expect(shouldInvalidateDocumentsList("stage_progress")).toBe(false);
  });

  it("invalidates on terminal / structural events", () => {
    expect(shouldInvalidateDocumentsList("ingestion_completed")).toBe(true);
    expect(shouldInvalidateDocumentsList("stage_completed")).toBe(true);
    expect(shouldInvalidateDocumentsList("StatusSnapshot")).toBe(true);
  });

  it("exports quieter debounce / safety-net windows", () => {
    expect(DOCUMENTS_INVALIDATE_DEBOUNCE_MS).toBe(1500);
    expect(DOCUMENTS_SAFETY_NET_INVALIDATE_MS).toBe(5000);
  });
});

describe("patchDocumentsCacheFromProgress", () => {
  it("patches matching row by track_id without touching others", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["documents", "ws-1"], {
      items: [makeDoc("a"), makeDoc("b")],
    });

    const patched = patchDocumentsCacheFromProgress(queryClient, {
      type: "stage_progress",
      track_id: "track-a",
      stage: "extracting",
      progress: 42,
      message: "Chunk 3/10",
    });

    expect(patched).toBe(1);
    const data = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(data?.items[0].current_stage).toBe("extracting");
    expect(data?.items[0].stage_progress).toBe(0.42);
    expect(data?.items[0].stage_message).toBe("Chunk 3/10");
    expect(data?.items[1].current_stage).toBeUndefined();
  });

  it("ignores ChunkProgress for cache patches", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["documents", "ws-1"], {
      items: [makeDoc("a")],
    });

    const patched = patchDocumentsCacheFromProgress(queryClient, {
      type: "ChunkProgress",
      track_id: "track-a",
      progress: 99,
    });

    expect(patched).toBe(0);
    const data = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(data?.items[0].stage_progress).toBeUndefined();
  });

  it("marks ingestion_completed as completed", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["documents", "ws-1"], {
      items: [makeDoc("a", { status: "processing", current_stage: "merging" })],
    });

    patchDocumentsCacheFromProgress(queryClient, {
      type: "ingestion_completed",
      track_id: "track-a",
      document_id: "a",
    });

    const data = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(data?.items[0].status).toBe("completed");
    expect(data?.items[0].stage_message).toBe("Completed");
  });

  it("SPEC-120: PdfPageProgress clears queued display_status", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["documents", "ws-1"], {
      items: [
        makeDoc("a", {
          status: "pending",
          current_stage: "queued",
          display_status: "queued",
          ui_phase: "idle",
          track_id: "pdf-track",
        }),
      ],
    });

    const patched = patchDocumentsCacheFromProgress(queryClient, {
      type: "PdfPageProgress",
      data: {
        document_id: "a",
        task_id: "pdf-track",
        current_page: 7,
        total_pages: 17,
        completed_pages: 7,
        progress: 0.41,
        phase: "ocr",
      },
    });

    expect(patched).toBe(1);
    const data = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(data?.items[0].current_stage).toBe("converting");
    expect(data?.items[0].display_status).toBe("converting");
    expect(data?.items[0].ui_phase).toBe("running");
    expect(data?.items[0].status).toBe("processing");
    expect(data?.items[0].stage_message).toBe("Converting 7/17 pages");
  });

  it("caps active PdfPageProgress below 1.0 and ignores late events after completed", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["documents", "ws-1"], {
      items: [
        makeDoc("a", {
          status: "processing",
          current_stage: "converting",
          stage_progress: 0.5,
          stage_message: "Converting 12/25 pages",
          track_id: "pdf-track",
        }),
      ],
    });

    patchDocumentsCacheFromProgress(queryClient, {
      type: "PdfPageProgress",
      data: {
        document_id: "a",
        task_id: "pdf-track",
        current_page: 25,
        total_pages: 25,
        completed_pages: 25,
        progress: 1,
        phase: "group_complete",
      },
    });
    let data = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(data?.items[0].stage_progress).toBe(0.99);

    patchDocumentsCacheFromProgress(queryClient, {
      type: "ingestion_completed",
      track_id: "pdf-track",
      document_id: "a",
    });
    patchDocumentsCacheFromProgress(queryClient, {
      type: "PdfPageProgress",
      data: {
        document_id: "a",
        task_id: "pdf-track",
        current_page: 3,
        total_pages: 25,
        completed_pages: 3,
        progress: 0.1,
        phase: "extracted",
      },
    });
    data = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(data?.items[0].status).toBe("completed");
    expect(data?.items[0].stage_progress).toBe(1);
  });

  it("patches StatusSnapshot active_tasks", () => {
    const queryClient = new QueryClient();
    queryClient.setQueryData(["documents", "ws-1"], {
      items: [makeDoc("a")],
    });

    const patched = patchDocumentsCacheFromProgress(queryClient, {
      type: "StatusSnapshot",
      active_tasks: [
        {
          track_id: "track-a",
          document_id: "a",
          status: "Extracting entities",
          progress: 55,
        },
      ],
    } as never);

    expect(patched).toBe(1);
    const data = queryClient.getQueryData<{ items: Document[] }>([
      "documents",
      "ws-1",
    ]);
    expect(data?.items[0].stage_progress).toBe(0.55);
    expect(data?.items[0].stage_message).toBe("Extracting entities");
  });
});
