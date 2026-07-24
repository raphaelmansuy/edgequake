import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  uploadDocument,
  uploadFile,
  uploadPdfDocument,
} from "@/lib/api/edgequake";

import { performFileUpload } from "../perform-file-upload";

vi.mock("@/lib/api/edgequake", () => ({
  uploadDocument: vi.fn(),
  uploadFile: vi.fn(),
  uploadPdfDocument: vi.fn(),
}));

function file(name: string, type: string): File {
  return new File(["payload"], name, { type });
}

describe("performFileUpload", () => {
  beforeEach(() => {
    vi.mocked(uploadDocument).mockReset();
    vi.mocked(uploadFile).mockReset();
    vi.mocked(uploadPdfDocument).mockReset();
  });

  it("routes PDF uploads with inline image analysis enabled by default", async () => {
    vi.mocked(uploadPdfDocument).mockResolvedValue({
      pdf_id: "pdf-1",
      document_id: "doc-1",
      status: "processing",
      task_id: "task-1",
      track_id: "track-1",
      message: "ok",
      estimated_time_seconds: 60,
      metadata: {
        filename: "paper.pdf",
        file_size_bytes: 100,
        page_count: 1,
        sha256_checksum: "abc",
        vision_enabled: true,
        vision_model: null,
      },
    });

    const pdf = file("paper.pdf", "application/pdf");
    await performFileUpload(pdf, { batchTrackId: "batch-1" });

    expect(uploadPdfDocument).toHaveBeenCalledWith(
      pdf,
      expect.objectContaining({
        analyze_inline_images: true,
        track_id: "batch-1",
      }),
    );
  });

  it("normalizes PDF progress track_id to server task_id (SPEC-054/#300)", async () => {
    vi.mocked(uploadPdfDocument).mockResolvedValue({
      pdf_id: "pdf-1",
      document_id: "doc-1",
      status: "queued",
      task_id: "pdf-server-task",
      track_id: "upload_client_batch",
      message: "ok",
      estimated_time_seconds: 60,
      metadata: {
        filename: "paper.pdf",
        file_size_bytes: 100,
        page_count: 1,
        sha256_checksum: "abc",
        vision_enabled: true,
        vision_model: null,
      },
    });

    const result = await performFileUpload(file("paper.pdf", "application/pdf"), {
      batchTrackId: "upload_client_batch",
    });

    expect(result.task_id).toBe("pdf-server-task");
    expect(result.track_id).toBe("pdf-server-task");
  });

  it("falls back to client track_id when legacy PDF response omits task_id", async () => {
    vi.mocked(uploadPdfDocument).mockResolvedValue({
      pdf_id: "pdf-1",
      document_id: "doc-1",
      status: "queued",
      task_id: "",
      track_id: "legacy-track",
      message: "ok",
      estimated_time_seconds: 60,
      metadata: {
        filename: "paper.pdf",
        file_size_bytes: 100,
        page_count: 1,
        sha256_checksum: "abc",
        vision_enabled: true,
        vision_model: null,
      },
    });

    const result = await performFileUpload(file("paper.pdf", "application/pdf"), {
      batchTrackId: "legacy-track",
    });

    expect(result.track_id).toBe("legacy-track");
  });

  it("routes PNG to multipart uploadFile, not JSON uploadDocument", async () => {
    vi.mocked(uploadFile).mockResolvedValue({
      document_id: "img-doc-1",
      status: "pending",
      track_id: "track-img",
      task_id: "task-img",
    });

    const png = file("diagram.png", "image/png");
    const result = await performFileUpload(png, { batchTrackId: "batch-1" });

    expect(uploadFile).toHaveBeenCalledWith(png, expect.anything());
    expect(uploadDocument).not.toHaveBeenCalled();
    expect(result.source_type).toBe("image");
    expect(result.document_id).toBe("img-doc-1");
    // Progress SSOT: prefer task_id for all upload kinds.
    expect(result.track_id).toBe("task-img");
  });

  it("routes markdown to uploadDocument and remaps progress to task_id", async () => {
    vi.mocked(uploadDocument).mockResolvedValue({
      document_id: "md-doc-1",
      status: "pending",
      // Legacy shape: client batch ≠ task (still prefer task_id)
      track_id: "batch-1",
      task_id: "insert-task-md",
    });

    const md = new File(["# Hello"], "notes.md", { type: "text/markdown" });
    const result = await performFileUpload(md, { batchTrackId: "batch-1" });

    expect(uploadDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "notes.md",
        source_type: "markdown",
        track_id: "batch-1",
      }),
    );
    expect(uploadFile).not.toHaveBeenCalled();
    expect(result.source_type).toBe("markdown");
    expect(result.track_id).toBe("insert-task-md");
  });

  it("068: when server aligns track_id with insert-* task_id, progress key matches", async () => {
    vi.mocked(uploadDocument).mockResolvedValue({
      document_id: "md-doc-2",
      status: "pending",
      track_id: "insert-aligned",
      task_id: "insert-aligned",
    });

    const md = new File(["# Hi"], "a.md", { type: "text/markdown" });
    const result = await performFileUpload(md, { batchTrackId: "batch-x" });
    expect(result.track_id).toBe("insert-aligned");
    expect(result.task_id).toBe("insert-aligned");
  });

  it("maps multipart duplicate_processing to duplicate_of", async () => {
    vi.mocked(uploadFile).mockResolvedValue({
      document_id: "existing-img",
      status: "duplicate_processing",
      is_duplicate: true,
    });

    const result = await performFileUpload(file("x.png", "image/png"), {
      batchTrackId: "batch-1",
    });

    expect(result.duplicate_of).toBe("existing-img");
  });
});
