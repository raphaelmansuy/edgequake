import { describe, expect, it } from "vitest";
import {
  mergeIngestionProgress,
  stageRank,
} from "@/lib/pipeline/merge-ingestion-progress";
import type { IngestionProgress, StageProgress } from "@/types/ingestion";

function makeProgress(opts: {
  stage?: string;
  status?: string;
  message?: string;
  pct?: number;
  updated_at?: string;
}): IngestionProgress {
  const stage = opts.stage ?? "uploading";
  const pct = opts.pct ?? 0;
  const stages: StageProgress[] = [
    {
      stage: stage as StageProgress["stage"],
      status: "running",
      progress: pct,
      total_items: 0,
      completed_items: 0,
    },
  ];
  return {
    track_id: "insert-1",
    document_id: "doc-1",
    document_name: "notes.md",
    status: opts.status ?? stage,
    overall_progress: pct,
    progress: {
      current_stage: stage as StageProgress["stage"],
      completion_percentage: pct,
      latest_message: opts.message ?? "Queued for processing…",
      stages,
    },
    started_at: "2026-01-01T00:00:00Z",
    updated_at: opts.updated_at ?? "2026-01-01T00:00:00Z",
  };
}

describe("mergeIngestionProgress (ux086_v_merge_rule)", () => {
  it("seed Queued loses to poll chunking@40", () => {
    const store = makeProgress({
      stage: "uploading",
      status: "pending",
      message: "Queued for processing…",
      pct: 0,
    });
    const poll = makeProgress({
      stage: "chunking",
      status: "chunking",
      message: "Chunking — Step 3",
      pct: 40,
      updated_at: "2026-01-01T00:01:00Z",
    });
    const merged = mergeIngestionProgress(store, poll)!;
    expect(merged.progress.current_stage).toBe("chunking");
    expect(merged.progress.latest_message).toMatch(/Chunking/);
    expect(merged.progress.completion_percentage).toBe(40);
  });

  it("same-stage lower poll keeps store (WS ahead)", () => {
    const store = makeProgress({
      stage: "extracting",
      status: "extracting",
      pct: 80,
      message: "Extracting — chunk 8/10",
    });
    const poll = makeProgress({
      stage: "extracting",
      status: "extracting",
      pct: 20,
      message: "Extracting entities",
    });
    const merged = mergeIngestionProgress(store, poll)!;
    expect(merged.progress.completion_percentage).toBe(80);
    expect(merged.progress.latest_message).toMatch(/chunk 8/);
  });

  it("terminal poll wins over running store", () => {
    const store = makeProgress({
      stage: "extracting",
      status: "extracting",
      pct: 50,
    });
    const poll = makeProgress({
      stage: "completed",
      status: "completed",
      pct: 100,
      message: "Done",
      updated_at: "2026-01-01T00:02:00Z",
    });
    const merged = mergeIngestionProgress(store, poll)!;
    expect(merged.status).toBe("completed");
  });

  it("cancelled terminal wins", () => {
    const store = makeProgress({
      stage: "chunking",
      status: "chunking",
      pct: 10,
    });
    const poll = makeProgress({
      stage: "cancelled",
      status: "cancelled",
      pct: 0,
      message: "Cancelled",
    });
    expect(mergeIngestionProgress(store, poll)?.status).toBe("cancelled");
  });

  it("stageRank orders converting before chunking", () => {
    expect(stageRank("converting")).toBeLessThan(stageRank("chunking"));
    expect(stageRank("pending")).toBeLessThan(stageRank("chunking"));
  });
});
