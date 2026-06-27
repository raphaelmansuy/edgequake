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

  it("routes PNG to multipart uploadFile, not JSON uploadDocument", async () => {
    vi.mocked(uploadFile).mockResolvedValue({
      document_id: "img-doc-1",
      status: "pending",
      track_id: "track-img",
      task_id: "task-img",
    });

    const png = file("diagram.png", "image/png");
    const result = await performFileUpload(png, { trackId: "batch-1" });

    expect(uploadFile).toHaveBeenCalledWith(png);
    expect(uploadDocument).not.toHaveBeenCalled();
    expect(result.source_type).toBe("image");
    expect(result.document_id).toBe("img-doc-1");
  });

  it("routes markdown to uploadDocument with text content", async () => {
    vi.mocked(uploadDocument).mockResolvedValue({
      document_id: "md-doc-1",
      status: "pending",
      track_id: "track-md",
    });

    const md = new File(["# Hello"], "notes.md", { type: "text/markdown" });
    const result = await performFileUpload(md, { trackId: "batch-1" });

    expect(uploadDocument).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "notes.md",
        source_type: "text",
        track_id: "batch-1",
      }),
    );
    expect(uploadFile).not.toHaveBeenCalled();
    expect(result.source_type).toBe("text");
  });

  it("maps multipart duplicate_processing to duplicate_of", async () => {
    vi.mocked(uploadFile).mockResolvedValue({
      document_id: "existing-img",
      status: "duplicate_processing",
      is_duplicate: true,
    });

    const result = await performFileUpload(file("x.png", "image/png"), {
      trackId: "batch-1",
    });

    expect(result.duplicate_of).toBe("existing-img");
  });
});
