/**
 * SPEC-086 — IngestionRunCard nesting rules + dismiss failed gate (no RTL).
 */
import { describe, expect, it } from "vitest";
import {
  isOrphanFailedAttention,
  partitionActiveRuns,
} from "@/components/documents/active-runs-panel";
import { canDismissFailedRun } from "@/components/documents/ingestion-run-card";
import type { IngestionRunView } from "@/lib/pipeline/ingestion-run-view";

/** Mirrors showPdfDetail gate in ingestion-run-card.tsx (keep in sync). */
function shouldShowPdfNestedDetail(
  run: Pick<IngestionRunView, "sourceType" | "stage">,
  hasNestedDetail: boolean,
): boolean {
  return (
    hasNestedDetail && run.sourceType === "pdf" && run.stage === "converting"
  );
}

describe("IngestionRunCard PDF nest gate (ux086_v_one_presenter)", () => {
  it("shows nest only for pdf + converting", () => {
    expect(
      shouldShowPdfNestedDetail(
        { sourceType: "pdf", stage: "converting" },
        true,
      ),
    ).toBe(true);
  });

  it("hides nest for markdown even if detail provided", () => {
    expect(
      shouldShowPdfNestedDetail(
        { sourceType: "markdown", stage: "chunking" },
        true,
      ),
    ).toBe(false);
  });

  it("hides nest for pdf after converting (chunking)", () => {
    expect(
      shouldShowPdfNestedDetail({ sourceType: "pdf", stage: "chunking" }, true),
    ).toBe(false);
  });
});

describe("IngestionRunCard dismiss failed (ux086 orphan staging)", () => {
  it("allows dismiss only for failed runs with handler", () => {
    expect(
      canDismissFailedRun({ stage: "failed", stageStatus: "failed" }, true),
    ).toBe(true);
    expect(
      canDismissFailedRun({ stage: "failed", stageStatus: "failed" }, false),
    ).toBe(false);
    expect(
      canDismissFailedRun({ stage: "extracting", stageStatus: "active" }, true),
    ).toBe(false);
  });

  it("classifies orphan re-upload failures for ActiveRuns dismiss", () => {
    const orphan: IngestionRunView = {
      documentId: "doc-1",
      trackId: "insert-dead",
      filename: "invarian.md",
      sourceType: "markdown",
      stage: "failed",
      stageStatus: "failed",
      message:
        "Prior interrupted upload — Upload interrupted during 'uploading'. Please re-upload the document.",
    };
    expect(isOrphanFailedAttention(orphan)).toBe(true);
    expect(canDismissFailedRun(orphan, true)).toBe(true);
  });
});

describe("ActiveRunsPanel partition (dual-run UX)", () => {
  const orphan: IngestionRunView = {
    documentId: "doc-orphan",
    trackId: "insert-dead",
    filename: "areal.md",
    sourceType: "markdown",
    stage: "failed",
    stageStatus: "failed",
    message: "Prior interrupted upload — please re-upload the document.",
  };
  const pdf: IngestionRunView = {
    documentId: "doc-pdf",
    trackId: "pdf-live",
    filename: "paper.pdf",
    sourceType: "pdf",
    stage: "converting",
    stageStatus: "active",
    message: "Converting PDF · 4/30 pages",
  };

  it("splits orphan failed shell from live PDF convert", () => {
    const { working, attention } = partitionActiveRuns([orphan, pdf]);
    expect(working.map((r) => r.documentId)).toEqual(["doc-pdf"]);
    expect(attention.map((r) => r.documentId)).toEqual(["doc-orphan"]);
  });

  it("keeps only attention when no live work", () => {
    const { working, attention } = partitionActiveRuns([orphan]);
    expect(working).toHaveLength(0);
    expect(attention).toHaveLength(1);
  });
});
