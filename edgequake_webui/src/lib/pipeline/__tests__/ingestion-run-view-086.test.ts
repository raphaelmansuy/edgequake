/**
 * SPEC-086 — buildIngestionRunViewFromProgress + converting skip for markdown.
 */
import { describe, expect, it } from "vitest";
import {
  buildIngestionRunView,
  buildIngestionRunViewFromProgress,
} from "@/lib/pipeline/ingestion-run-view";
import { buildStageTimeline } from "@/lib/pipeline/stage-timeline";
import type { Document } from "@/types";
import type { IngestionProgress } from "@/types/ingestion";

function progress(stage: string): IngestionProgress {
  return {
    track_id: "insert-1",
    document_id: "doc-1",
    document_name: "notes.md",
    status: stage,
    overall_progress: 40,
    progress: {
      current_stage: stage as never,
      completion_percentage: 40,
      latest_message: "Chunking — 2/5",
      stages: [],
    },
    started_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:01:00Z",
  };
}

describe("buildIngestionRunViewFromProgress (ux086_v_one_presenter)", () => {
  it("maps progress to run view for markdown", () => {
    const run = buildIngestionRunViewFromProgress(progress("chunking"), {
      sourceType: "markdown",
      filename: "notes.md",
    });
    expect(run.stage).toBe("chunking");
    expect(run.sourceType).toBe("markdown");
    expect(run.filename).toBe("notes.md");
    expect(run.counts?.current).toBe(2);
    expect(run.counts?.total).toBe(5);
  });

  it("skips converting for markdown timeline", () => {
    const run = buildIngestionRunViewFromProgress(progress("chunking"), {
      sourceType: "markdown",
    });
    const timeline = buildStageTimeline(run);
    const converting = timeline.steps.find((s) => s.id === "converting");
    expect(converting).toBeUndefined();
  });

  it("keeps converting for pdf", () => {
    const run = buildIngestionRunViewFromProgress(progress("converting"), {
      sourceType: "pdf",
      filename: "paper.pdf",
    });
    const timeline = buildStageTimeline(run);
    const converting = timeline.steps.find((s) => s.id === "converting");
    expect(converting).toBeTruthy();
    expect(converting?.status).not.toBe("skipped");
    expect(converting?.label).toBe("Converting PDF");
  });

  it("infers markdown from .md filename when source_type missing", () => {
    const doc = {
      id: "doc-md",
      file_name: "invarian_2607.11875v2.md",
      status: "pending",
      current_stage: "chunking",
      stage_message: "Chunking — 1/3",
      track_id: "insert-1",
    } as Document;
    const run = buildIngestionRunView(doc);
    expect(run?.sourceType).toBe("markdown");
    const timeline = buildStageTimeline(run!);
    const converting = timeline.steps.find((s) => s.id === "converting");
    expect(converting).toBeUndefined();
  });

  it("projects aged orphan uploading shell as failed re-upload", () => {
    const doc = {
      id: "doc-orphan",
      file_name: "invarian_2607.11875v2.md",
      status: "pending",
      current_stage: "uploading",
      stage_message: "Document received, starting processing",
      stage_progress: 0,
      track_id: "insert-dead",
      admission_staging: true,
      updated_at: "2020-01-01T00:00:00Z",
      created_at: "2020-01-01T00:00:00Z",
    } as Document;
    const run = buildIngestionRunView(doc);
    expect(run?.stage).toBe("failed");
    expect(run?.stageStatus).toBe("failed");
    expect(run?.message.toLowerCase()).toMatch(
      /prior interrupted|re-upload|interrupted|document received/,
    );
  });
});
