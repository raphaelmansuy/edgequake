import { describe, expect, it } from "vitest";

import {
  documentStageRank,
  getDocumentDisplayStatus,
  isProcessingStatus,
  isTerminalStatus,
  normalizeDocumentStageProgress,
  normalizeProgress01,
  normalizeStatus,
} from "../status-domain";

describe("document status domain", () => {
  it("normalizes legacy status text without UI dependencies", () => {
    expect(normalizeStatus("  ChUnKiNg ")).toBe("chunking");
    expect(normalizeStatus("still_processing")).toBe("processing");
    expect(normalizeStatus("unknown")).toBe("pending");
  });

  it("lets terminal document state beat a stale current stage", () => {
    expect(
      getDocumentDisplayStatus({
        status: "completed",
        current_stage: "extracting",
      }),
    ).toBe("completed");
    expect(isTerminalStatus("completed")).toBe(true);
    expect(isProcessingStatus("completed")).toBe(false);
  });

  it("does not derive document status from task-only presentation", () => {
    const document = {
      status: "processing",
      current_stage: "converting",
      presentation: { badge: "Needs attention" },
    };
    expect(getDocumentDisplayStatus(document)).toBe("converting");
  });

  it("normalizes all wire progress variants to 0..1", () => {
    expect(normalizeProgress01(0.41)).toBe(0.41);
    expect(normalizeProgress01(41)).toBe(0.41);
    expect(normalizeProgress01(150)).toBe(1);
    expect(normalizeProgress01(-2)).toBe(0);
    expect(normalizeProgress01(Number.NaN)).toBeUndefined();
    expect(
      normalizeDocumentStageProgress({ stage_progress: 55 }).stage_progress,
    ).toBe(0.55);
  });

  it("uses the backend-compatible stable stage order", () => {
    expect(documentStageRank("queued")).toBeLessThan(
      documentStageRank("converting"),
    );
    expect(documentStageRank("converting")).toBeLessThan(
      documentStageRank("extracting"),
    );
    expect(documentStageRank("extracting")).toBeLessThan(
      documentStageRank("embedding"),
    );
    expect(documentStageRank("embedding")).toBeLessThan(
      documentStageRank("projecting"),
    );
    expect(documentStageRank("projecting")).toBeLessThan(
      documentStageRank("completed"),
    );
    expect(normalizeStatus("projecting")).toBe("projecting");
    expect(isProcessingStatus("projecting")).toBe(true);
  });

  it("honors cancelRequested dual-SSOT over in-flight stage", () => {
    expect(
      getDocumentDisplayStatus(
        { status: "embedding", current_stage: "embedding" },
        { cancelRequested: true },
      ),
    ).toBe("stopping");
  });

  it("treats delete_failed / dead_letter as terminal lifecycle statuses", () => {
    expect(isTerminalStatus("delete_failed")).toBe(true);
    expect(isTerminalStatus("dead_letter")).toBe(true);
    expect(isProcessingStatus("cancelling")).toBe(true);
    expect(
      getDocumentDisplayStatus({
        status: "delete_failed",
        current_stage: "delete_failed",
      }),
    ).toBe("delete_failed");
  });

  it("recognizes held as a known normalized status", () => {
    expect(normalizeStatus("held")).toBe("held");
    expect(normalizeStatus("cancelling")).toBe("cancelling");
  });
});
