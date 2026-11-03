import { describe, expect, it } from "vitest";

import {
  classifyUploadFile,
  isImageUploadFile,
  isMarkdownUploadFile,
  isPdfUploadFile,
  sourceTypeFromFileName,
  splitByImageCapability,
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

  it("does not classify Office as pdf or image (SPEC-121)", () => {
    expect(isPdfUploadFile(file("memo.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"))).toBe(
      false,
    );
    expect(isImageUploadFile(file("sheet.xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"))).toBe(
      false,
    );
    // Forced past dropzone Accept, router would treat as text — API still rejects.
    expect(classifyUploadFile(file("memo.docx", ""))).toBe("text");
  });
});

describe("SPEC-183 splitByImageCapability", () => {
  it("separates pictures from everything else, so a text-only workspace can refuse only them", () => {
    const files = [
      file("policy.md", "text/markdown"),
      file("scan.png", "image/png"),
      file("handbook.pdf", "application/pdf"),
      file("photo.JPG", ""),
    ];
    const { text, images } = splitByImageCapability(files);
    expect(text.map((f) => f.name)).toEqual(["policy.md", "handbook.pdf"]);
    expect(images.map((f) => f.name)).toEqual(["scan.png", "photo.JPG"]);
  });

  it("returns everything as text when nothing is a picture", () => {
    const { text, images } = splitByImageCapability([file("a.txt", "text/plain")]);
    expect(images).toEqual([]);
    expect(text).toHaveLength(1);
  });

  it("handles an empty drop", () => {
    expect(splitByImageCapability([])).toEqual({ text: [], images: [] });
  });
});
