import { describe, expect, it } from "vitest";
import {
  buildIngestionRunView,
  type IngestionRunView,
} from "@/lib/pipeline/ingestion-run-view";
import {
  buildStageTimeline,
  expectedUnitForStage,
  formatStepDetailLine,
} from "@/lib/pipeline/stage-timeline";
import type { Document } from "@/types";

function run(partial: Partial<IngestionRunView>): IngestionRunView {
  return {
    documentId: "d1",
    trackId: "t1",
    filename: "doc.pdf",
    sourceType: "pdf",
    stage: "extracting",
    stageStatus: "active",
    message: "Extracting",
    ...partial,
  };
}

function doc(partial: Partial<Document> & { id: string }): Document {
  return {
    title: partial.title ?? partial.file_name ?? partial.id,
    chunk_count: 0,
    ...partial,
  } as Document;
}

describe("stage-timeline", () => {
  it("omits converting for markdown (no Converting PDF label)", () => {
    const tl = buildStageTimeline(
      run({ sourceType: "markdown", stage: "chunking" }),
    );
    const converting = tl.steps.find((s) => s.id === "converting");
    expect(converting).toBeUndefined();
    expect(tl.steps.find((s) => s.id === "chunking")?.status).toBe("active");
  });

  it("marks prior stages done and attaches detail on active extracting", () => {
    const tl = buildStageTimeline(
      run({
        stage: "extracting",
        counts: { current: 42, total: 351, unit: "chunks" },
        progress01: 0.12,
        message: "Extracting entities — chunk 42/351",
      }),
    );
    expect(tl.steps.find((s) => s.id === "uploading")?.status).toBe("done");
    expect(tl.steps.find((s) => s.id === "chunking")?.status).toBe("done");
    const extracting = tl.steps.find((s) => s.id === "extracting");
    expect(extracting?.status).toBe("active");
    expect(extracting?.detail?.current).toBe(42);
    expect(extracting?.detail?.total).toBe(351);
    expect(formatStepDetailLine(extracting?.detail)).toContain("42/351");
    expect(tl.admissionQueued).toBe(false);
  });

  it("queued admission: all processing steps pending", () => {
    const tl = buildStageTimeline(
      run({ stage: "queued", stageStatus: "pending", trackId: null }),
    );
    expect(tl.admissionQueued).toBe(true);
    expect(tl.admissionCleaning).toBe(false);
    expect(tl.admissionPhase).toBe("queued");
    expect(tl.overallProgress01).toBe(0);
    expect(tl.steps.every((s) => s.status === "pending" || s.status === "skipped")).toBe(
      true,
    );
  });

  it("cleaning admission: distinct from queued, progress stays 0", () => {
    const tl = buildStageTimeline(
      run({
        stage: "cleaning",
        stageStatus: "pending",
        trackId: null,
        message: "Removing prior knowledge graph data…",
      }),
    );
    expect(tl.admissionCleaning).toBe(true);
    expect(tl.admissionQueued).toBe(false);
    expect(tl.admissionPhase).toBe("cleaning");
    expect(tl.overallProgress01).toBe(0);
    expect(tl.steps.every((s) => s.status === "pending" || s.status === "skipped")).toBe(
      true,
    );
  });

  it("never shows 100% overall while converting even if stage_progress=1", () => {
    const tl = buildStageTimeline(
      run({
        stage: "converting",
        progress01: 1,
        message: "Converting PDF",
      }),
    );
    expect(tl.overallProgress01).toBeLessThan(0.5);
    expect(tl.overallProgress01).toBeGreaterThan(0);
    expect(tl.overallIsEstimate).toBe(true);
  });

  it("weights extracting higher than uploading in overall estimate", () => {
    const early = buildStageTimeline(
      run({ stage: "uploading", progress01: 0.5 }),
    );
    const late = buildStageTimeline(
      run({
        stage: "extracting",
        counts: { current: 175, total: 350, unit: "chunks" },
        progress01: 0.5,
      }),
    );
    expect(late.overallProgress01).toBeGreaterThan(early.overallProgress01);
  });

  it("only reaches 100% when completed", () => {
    const mid = buildStageTimeline(
      run({
        stage: "storing",
        progress01: 0.99,
        counts: { current: 99, total: 100, unit: "relationships" },
      }),
    );
    expect(mid.overallProgress01).toBeLessThan(1);
    const done = buildStageTimeline(
      run({ stage: "completed", stageStatus: "complete" }),
    );
    expect(done.overallProgress01).toBe(1);
  });

  it("merge mode skips stages before merging", () => {
    const tl = buildStageTimeline(
      run({
        stage: "merging",
        mode: "merge",
        counts: { current: 10, total: 100, unit: "entities" },
        progress01: 0.1,
        message: "Merging 10/100 entities",
      }),
    );
    expect(tl.steps.find((s) => s.id === "extracting")?.status).toBe("skipped");
    expect(tl.steps.find((s) => s.id === "chunking")?.status).toBe("skipped");
    expect(tl.steps.find((s) => s.id === "merging")?.status).toBe("active");
    expect(tl.steps.find((s) => s.id === "embedding")?.status).toBe("pending");
  });

  it("entities mode skips uploading and converting", () => {
    // Default sourceType pdf: converting stays in timeline but skipped for entities mode.
    const tl = buildStageTimeline(
      run({
        stage: "extracting",
        mode: "entities",
        counts: { current: 1, total: 10, unit: "chunks" },
      }),
    );
    expect(tl.steps.find((s) => s.id === "uploading")?.status).toBe("skipped");
    expect(tl.steps.find((s) => s.id === "converting")?.status).toBe("skipped");
    expect(tl.steps.find((s) => s.id === "extracting")?.status).toBe("active");

    const mdEntities = buildStageTimeline(
      run({
        stage: "extracting",
        mode: "entities",
        sourceType: "markdown",
        filename: "notes.md",
        counts: { current: 1, total: 10, unit: "chunks" },
      }),
    );
    expect(mdEntities.steps.find((s) => s.id === "converting")).toBeUndefined();
  });

  it("gleaning is a first-class active step after extracting done", () => {
    const tl = buildStageTimeline(
      run({
        stage: "gleaning",
        counts: { current: 5, total: 20, unit: "chunks" },
        progress01: 0.25,
        message: "Gleaning chunk 5/20",
      }),
    );
    expect(tl.steps.find((s) => s.id === "extracting")?.status).toBe("done");
    expect(tl.steps.find((s) => s.id === "gleaning")?.status).toBe("active");
  });

  it("failed mid-extract marks extracting failed and later pending", () => {
    const tl = buildStageTimeline(
      run({
        stage: "extracting",
        stageStatus: "failed",
        counts: { current: 12, total: 100, unit: "chunks" },
        message: "12 chunks failed · Retry available",
      }),
    );
    expect(tl.steps.find((s) => s.id === "chunking")?.status).toBe("done");
    expect(tl.steps.find((s) => s.id === "extracting")?.status).toBe("failed");
    expect(tl.steps.find((s) => s.id === "embedding")?.status).toBe("pending");
  });

  it("completed marks all non-skipped steps done", () => {
    const tl = buildStageTimeline(
      run({
        stage: "completed",
        stageStatus: "complete",
        sourceType: "text",
      }),
    );
    expect(tl.steps.find((s) => s.id === "converting")).toBeUndefined();
    expect(
      tl.steps
        .filter((s) => s.status !== "skipped")
        .every((s) => s.status === "done"),
    ).toBe(true);
    expect(tl.overallProgress01).toBe(1);
  });

  it("expected units match countable stages", () => {
    expect(expectedUnitForStage("extracting")).toBe("chunks");
    expect(expectedUnitForStage("converting")).toBe("pages");
    expect(expectedUnitForStage("merging")).toBe("entities");
  });

  it("buildIngestionRunView reads reprocess_mode", () => {
    const view = buildIngestionRunView(
      doc({
        id: "d1",
        status: "processing",
        current_stage: "merging",
        reprocess_mode: "merge",
        source_type: "pdf",
        file_name: "a.pdf",
      }),
    );
    expect(view?.mode).toBe("merge");
    const tl = buildStageTimeline(view!);
    expect(tl.steps.find((s) => s.id === "extracting")?.status).toBe("skipped");
  });
});
