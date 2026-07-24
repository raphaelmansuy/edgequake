/**
 * SPEC-086 — applyPolledProgress advances store; seed must not stick.
 */
import { act } from "react";
import { beforeEach, describe, expect, it } from "vitest";
import { useIngestionStore } from "@/stores/use-ingestion-store";
import type { IngestionProgress } from "@/types/ingestion";

function pollChunking(): IngestionProgress {
  return {
    track_id: "insert-086",
    document_id: "doc-086",
    document_name: "notes.md",
    status: "chunking",
    overall_progress: 40,
    progress: {
      current_stage: "chunking",
      completion_percentage: 40,
      latest_message: "Chunking — Step 3",
      stages: [
        {
          stage: "chunking",
          status: "running",
          progress: 40,
          total_items: 0,
          completed_items: 0,
        },
      ],
    },
    started_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:01:00Z",
  };
}

describe("useIngestionProgress 086 merge contract", () => {
  beforeEach(() => {
    useIngestionStore.getState().clearAllTracks();
  });

  it("applyPolledProgress advances seeded Queued to chunking", () => {
    const store = useIngestionStore.getState();
    act(() => {
      store.startTracking("insert-086", "doc-086", "notes.md");
    });
    expect(
      useIngestionStore.getState().tracks.get("insert-086")?.progress
        .latest_message,
    ).toMatch(/Queued/i);

    act(() => {
      useIngestionStore.getState().applyPolledProgress(pollChunking());
    });

    const track = useIngestionStore.getState().tracks.get("insert-086");
    expect(track?.progress.current_stage).toBe("chunking");
    expect(track?.progress.latest_message).toMatch(/Chunking/);
    expect(track?.progress.completion_percentage).toBe(40);
  });

  it("tracks Map replace is observable via selector", () => {
    act(() => {
      useIngestionStore
        .getState()
        .startTracking("insert-086", "doc-086", "notes.md");
      useIngestionStore.getState().applyPolledProgress(pollChunking());
    });
    const a = useIngestionStore.getState().tracks.get("insert-086");
    act(() => {
      const base = pollChunking();
      useIngestionStore.getState().applyPolledProgress({
        ...base,
        overall_progress: 70,
        updated_at: "2026-01-01T00:02:00Z",
        progress: {
          ...base.progress,
          completion_percentage: 70,
          latest_message: "Chunking 70%",
          stages: [
            {
              stage: "chunking",
              status: "running",
              progress: 70,
              total_items: 0,
              completed_items: 0,
            },
          ],
        },
      });
    });
    const b = useIngestionStore.getState().tracks.get("insert-086");
    expect(b).not.toBe(a);
    expect(b?.progress.completion_percentage).toBe(70);
  });

  it("StageTransition WS advances track", () => {
    act(() => {
      useIngestionStore
        .getState()
        .startTracking("insert-086", "doc-086", "notes.md");
      useIngestionStore.getState().updateFromMessage({
        type: "StageTransition",
        data: {
          document_id: "doc-086",
          task_id: "insert-086",
          stage: "extracting",
          stage_message: "Extracting entities",
          stage_progress: 0.1,
        },
      });
    });
    const track = useIngestionStore.getState().tracks.get("insert-086");
    expect(track?.progress.current_stage).toBe("extracting");
    expect(track?.progress.latest_message).toMatch(/Extracting/);
  });
});
