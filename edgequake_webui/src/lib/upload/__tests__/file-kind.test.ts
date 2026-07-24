import { describe, expect, it } from "vitest";

import {
  classifyUploadFile,
  isImageUploadFile,
  isMarkdownUploadFile,
  isPdfUploadFile,
  sourceTypeFromFileName,
} from "../file-kind";

function file(name: string, type: string): File {
  return new File(["content"], name, { type });
}

describe("file-kind", () => {
  it("classifies PDF by mime and extension", () => {
    expect(isPdfUploadFile(file("doc.pdf", "application/pdf"))).toBe(true);
    expect(isPdfUploadFile(file("doc.PDF", ""))).toBe(true);
    expect(classifyUploadFile(file("doc.pdf", "application/pdf"))).toBe("pdf");
  });

  it("classifies images by mime and extension", () => {
    expect(isImageUploadFile(file("scan.png", "image/png"))).toBe(true);
    expect(isImageUploadFile(file("photo.jpg", "image/jpeg"))).toBe(true);
    expect(isImageUploadFile(file("photo.jpeg", ""))).toBe(true);
    expect(isImageUploadFile(file("anim.gif", "image/gif"))).toBe(true);
    expect(isImageUploadFile(file("shot.webp", "image/webp"))).toBe(true);
    expect(classifyUploadFile(file("scan.png", "image/png"))).toBe("image");
  });

  it("classifies text files as text route", () => {
    expect(classifyUploadFile(file("notes.md", "text/markdown"))).toBe("text");
    expect(classifyUploadFile(file("data.json", "application/json"))).toBe(
      "text",
    );
  });

  it("detects markdown for source_type taxonomy (SPEC-086)", () => {
    expect(isMarkdownUploadFile(file("notes.md", "text/markdown"))).toBe(true);
    expect(isMarkdownUploadFile(file("NOTES.MD", ""))).toBe(true);
    expect(isMarkdownUploadFile(file("notes.txt", "text/plain"))).toBe(false);
  });

  it("sourceTypeFromFileName distinguishes md vs txt vs pdf", () => {
    expect(sourceTypeFromFileName("notes.md")).toBe("markdown");
    expect(sourceTypeFromFileName("notes.txt")).toBe("text");
    expect(sourceTypeFromFileName("paper.pdf")).toBe("pdf");
    expect(sourceTypeFromFileName("scan.png")).toBe("image");
  });

  it("does not treat PDF as image", () => {
    expect(isImageUploadFile(file("doc.pdf", "application/pdf"))).toBe(false);
  });
});
