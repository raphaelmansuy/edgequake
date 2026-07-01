import { describe, expect, it } from "bun:test";
import { extractPdfPageCount } from "@/lib/pdf/extract-page-count";

describe("extractPdfPageCount", () => {
  it("finds largest /Count in PDF bytes", () => {
    const pdf = new TextEncoder().encode(
      "%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Count 603/Kids[]>>endobj\n",
    );
    expect(extractPdfPageCount(pdf)).toBe(603);
  });

  it("returns null when no count token", () => {
    const pdf = new TextEncoder().encode("%PDF-1.4\n");
    expect(extractPdfPageCount(pdf)).toBeNull();
  });
});
