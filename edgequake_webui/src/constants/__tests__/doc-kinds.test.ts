import { describe, expect, it } from "vitest";

import { DOC_KINDS } from "../rag-profile";

/**
 * SPEC-184 — a filter chip must name a kind the product can actually ingest.
 *
 * WHY this is a test and not a comment: the `Office` chip shipped for months. It rendered, it
 * toggled, and clicking it filtered any workspace to zero results, because DOCX/XLSX/PPTX are a
 * v1 non-goal (LAW-121-8 in SPEC-121) — nothing in the ingest path converts a word-processor file.
 * A control that cannot return a result is worse than a missing control: it reads as a broken
 * search rather than as a format the product does not accept.
 *
 * The supported matrix is published in `specs/121-pdf-docx/09-acceptance.md`: TXT, MD, JSON, PDF
 * and PNG/JPG/GIF/WEBP. Those map onto exactly these four chips.
 */
describe("SPEC-184 document-kind chips match the ingestible matrix", () => {
  it("offers only the kinds the server can store", () => {
    expect([...DOC_KINDS]).toEqual(["pdf", "image", "markdown", "text"]);
  });

  it("never offers Office, which has no ingest path in v1", () => {
    // When an Office adapter lands it converts to Markdown and reuses text ingest, so the kind
    // would arrive as `markdown` — not as a fifth chip.
    expect(DOC_KINDS as readonly string[]).not.toContain("office");
  });

  it("covers JSON and CSV under text, which is how the server classifies them", () => {
    // `text/csv` maps to DocKind::Text server-side, so a data-file corpus is filterable as Text
    // rather than needing a kind of its own.
    expect(DOC_KINDS as readonly string[]).toContain("text");
  });
});
