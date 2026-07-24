import { describe, expect, it } from "vitest";

import { shouldUsePdfReprocessPanel } from "../use-reprocess-tracking";

describe("shouldUsePdfReprocessPanel", () => {
  it("uses PdfUploadProgress only for full PDF reprocess", () => {
    expect(shouldUsePdfReprocessPanel(true, "full")).toBe(true);
  });

  it("uses IngestionRunCard (no PDF nest) for entities-only PDF", () => {
    expect(shouldUsePdfReprocessPanel(true, "entities")).toBe(false);
  });

  it("uses IngestionRunCard (no PDF nest) for non-PDF full mode", () => {
    expect(shouldUsePdfReprocessPanel(false, "full")).toBe(false);
  });

  it("defaults safely when mode is missing", () => {
    expect(shouldUsePdfReprocessPanel(true, undefined)).toBe(false);
  });
});
