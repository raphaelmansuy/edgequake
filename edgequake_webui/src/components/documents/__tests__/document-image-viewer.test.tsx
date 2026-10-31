/**
 * SPEC-180 — an image document must render as a picture.
 *
 * The bug this pins: image documents store their file name as `content`, so the detail page
 * painted "Screenshot_…png" inside a markdown card instead of showing the image.
 */

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { isImageDocument } from "@/components/documents/document-image-viewer";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (_key: string, fallback: string) => fallback,
  }),
}));

const fetchMock = vi.fn();
const readyImage = { status: "ready", url: "blob:preview" } as const;
let imageState: typeof readyImage | { status: "loading" } | { status: "failed" } = readyImage;

vi.mock("@/hooks/use-authed-image-url", () => ({
  useAuthedImageUrl: () => imageState,
}));

vi.mock("@/lib/api/edgequake", () => ({
  getDocumentOriginalDownloadUrl: (id: string) =>
    `http://api.test/api/v1/documents/${id}/download/original`,
}));

import { DocumentImageViewer } from "@/components/documents/document-image-viewer";

const imageDoc = {
  id: "doc-img-1",
  title: "Screenshot 2026-10-31",
  file_name: "Screenshot_2026-10-31_at_3.30.53_PM__2_.png",
  mime_type: "image/png",
  file_size: 245_760,
};

describe("isImageDocument", () => {
  it("recognises the typed source", () => {
    expect(isImageDocument({ source_type: "image" })).toBe(true);
  });

  it("recognises an image MIME type", () => {
    expect(isImageDocument({ mime_type: "image/jpeg" })).toBe(true);
    expect(isImageDocument({ mime_type: "image/webp" })).toBe(true);
  });

  it("recognises an image extension when type is missing", () => {
    expect(isImageDocument({ file_name: "photo.JPG" })).toBe(true);
    expect(isImageDocument({ title: "scan.png" })).toBe(true);
  });

  it("does not claim text, markdown or PDF documents", () => {
    expect(isImageDocument({ source_type: "pdf", mime_type: "application/pdf" })).toBe(false);
    expect(isImageDocument({ source_type: "markdown", file_name: "notes.md" })).toBe(false);
    expect(isImageDocument({ source_type: "text", file_name: "notes.txt" })).toBe(false);
    expect(isImageDocument({})).toBe(false);
  });
});

describe("DocumentImageViewer", () => {
  beforeEach(() => {
    imageState = readyImage;
    fetchMock.mockReset();
  });

  it("shows the picture, not its filename as body text", () => {
    const { container } = render(<DocumentImageViewer document={imageDoc} />);
    const img = screen.getByTestId("document-image") as HTMLImageElement;
    expect(img.getAttribute("src")).toBe("blob:preview");
    expect(img.getAttribute("alt")).toBeTruthy();
    // The filename may appear as a toolbar label, but it must never be the body text.
    const stage = container.querySelector("[data-testid='document-image-stage']");
    expect(stage?.textContent ?? "").not.toMatch(/\.png\s*$/);
    expect(screen.getByTitle(imageDoc.file_name)).toBeTruthy();
  });

  it("offers zoom, fit, download and open", () => {
    render(<DocumentImageViewer document={imageDoc} />);
    expect(screen.getByTestId("document-image-zoom-in")).toBeTruthy();
    expect(screen.getByTestId("document-image-zoom-out")).toBeTruthy();
    expect(screen.getByTestId("document-image-fit")).toBeTruthy();
    expect(screen.getByTestId("document-image-download")).toBeTruthy();
    expect(screen.getByTestId("document-image-open")).toBeTruthy();
  });

  it("starts at 100% and fit is a no-op at that zoom", () => {
    render(<DocumentImageViewer document={imageDoc} />);
    expect(screen.getByTestId("document-image-zoom-level").textContent).toBe("100%");
    expect(screen.getByTestId("document-image-fit").hasAttribute("disabled")).toBe(true);
  });

  it("shows a loading state while the protected bytes are fetched", () => {
    imageState = { status: "loading" };
    render(<DocumentImageViewer document={imageDoc} />);
    expect(screen.queryByTestId("document-image")).toBeNull();
  });

  it("explains a failed load instead of showing a broken image", () => {
    imageState = { status: "failed" };
    render(<DocumentImageViewer document={imageDoc} />);
    expect(screen.getByTestId("document-image-failed")).toBeTruthy();
    expect(screen.getByText(/could not be loaded/i)).toBeTruthy();
  });

  it("links the download to the original bytes endpoint", () => {
    render(<DocumentImageViewer document={imageDoc} />);
    const link = screen.getByTestId("document-image-download") as HTMLAnchorElement;
    expect(link.getAttribute("href")).toContain("/download/original");
    expect(link.getAttribute("download")).toBe(imageDoc.file_name);
  });
});
