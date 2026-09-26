/**
 * SPEC-149 — progress event normalizer (LAW-149-6).
 */
import { describe, expect, it } from "vitest";
import { normalizeProgressEvent } from "../progress-event-normalizer";

describe("normalizeProgressEvent", () => {
  it("U-149-10 maps PdfPageProgress page_num → current_page and derives progress", () => {
    const event = normalizeProgressEvent({
      type: "PdfPageProgress",
      data: {
        pdf_id: "pdf-1",
        task_id: "track-1",
        page_num: 5,
        total_pages: 10,
        completed_pages: 5,
        phase: "extraction",
        markdown_len: 100,
        success: true,
        error: null,
      },
    });

    expect(event?.type).toBe("PdfPageProgress");
    if (event?.type !== "PdfPageProgress") return;
    expect(event.data.document_id).toBe("pdf-1");
    expect(event.data.current_page).toBe(5);
    expect(event.data.total_pages).toBe(10);
    expect(event.data.completed_pages).toBe(5);
    expect(event.data.progress).toBe(0.5);
    expect(event.data.phase).toBe("extraction");
  });

  it("derives progress from completed_pages not physical page_num", () => {
    const event = normalizeProgressEvent({
      type: "PdfPageProgress",
      data: {
        pdf_id: "pdf-1",
        task_id: "track-1",
        page_num: 24,
        total_pages: 25,
        completed_pages: 12,
        phase: "extracted",
        success: true,
      },
    });
    expect(event?.type).toBe("PdfPageProgress");
    if (event?.type !== "PdfPageProgress") return;
    expect(event.data.current_page).toBe(24);
    expect(event.data.completed_pages).toBe(12);
    expect(event.data.progress).toBe(0.48);
  });

  it("legacy progress without completed_pages derives completion count", () => {
    const event = normalizeProgressEvent({
      type: "PdfPageProgress",
      data: {
        pdf_id: "pdf-1",
        task_id: "track-1",
        page_num: 7,
        total_pages: 10,
        progress: 0.7,
        phase: "extraction",
        success: true,
      },
    });
    expect(event?.type).toBe("PdfPageProgress");
    if (event?.type !== "PdfPageProgress") return;
    expect(event.data.completed_pages).toBe(7);
    expect(event.data.progress).toBe(0.7);
  });

  it("clamps malformed progress into 0..1", () => {
    const event = normalizeProgressEvent({
      type: "PdfPageProgress",
      data: {
        pdf_id: "pdf-1",
        task_id: "track-1",
        page_num: 1,
        total_pages: 10,
        completed_pages: 1,
        progress: 250,
      },
    });
    expect(event?.type).toBe("PdfPageProgress");
    if (event?.type !== "PdfPageProgress") return;
    expect(event.data.progress).toBe(1);
  });

  it("U-149-11 preserves StageTransition data envelope", () => {
    const event = normalizeProgressEvent({
      type: "StageTransition",
      data: {
        document_id: "doc-1",
        task_id: "track-1",
        stage: "extracting",
        stage_message: "Extracting entities",
        stage_progress: 0.4,
      },
    });

    expect(event).toEqual({
      type: "StageTransition",
      data: {
        document_id: "doc-1",
        task_id: "track-1",
        stage: "extracting",
        stage_message: "Extracting entities",
        stage_progress: 0.4,
      },
    });
  });

  it("normalizes GraphStorageProgress", () => {
    const event = normalizeProgressEvent({
      type: "GraphStorageProgress",
      data: {
        track_id: "t1",
        document_id: "d1",
        sub_phase: "merge",
        sub_phase_label: "Merging",
        entities_processed: 1,
        entities_total: 2,
        entities_created: 1,
        entities_updated: 0,
        relationships_processed: 0,
        relationships_total: 0,
        relationships_created: 0,
        relationships_updated: 0,
        elapsed_ms: 10,
        eta_ms: null,
      },
    });
    expect(event?.type).toBe("GraphStorageProgress");
  });

  it("returns null for unknown types", () => {
    expect(normalizeProgressEvent({ type: "TotallyUnknown" })).toBeNull();
  });
});
