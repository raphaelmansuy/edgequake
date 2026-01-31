/**
 * OODA-44: Ingestion Store Unit Tests
 *
 * @description Comprehensive tests for useIngestionStore Zustand store
 * @implements Phase 4 Testing & Validation
 * @see specs/001-upload-pdf.md
 */

import type { CostUpdateEvent } from "@/types/cost";
import type {
  IngestionCompletedEvent,
  IngestionFailedEvent,
  IngestionStage,
  IngestionStartedEvent,
  StageCompletedEvent,
  StageProgressEvent,
  StageStartedEvent,
} from "@/types/ingestion";
import { act } from "react";
import { beforeEach, describe, expect, it } from "vitest";
import { useIngestionStore } from "../use-ingestion-store";

// Reset store before each test
beforeEach(() => {
  const store = useIngestionStore.getState();
  store.clearAllTracks();
  store.clearCompletedJobs();
  store.clearAllFailedJobs();
  store.setWsConnected(false);
  store.setWsReconnecting(false);
});

// ============================================================================
// Track Management Tests
// ============================================================================

describe("Track Management", () => {
  describe("startTracking", () => {
    it("should create initial progress for new track", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const track = store.getTrack("track-1");
      expect(track).toBeDefined();
      expect(track?.track_id).toBe("track-1");
      expect(track?.document_id).toBe("doc-1");
      expect(track?.document_name).toBe("test.pdf");
      expect(track?.status).toBe("pending");
      expect(track?.overall_progress).toBe(0);
    });

    it("should not overwrite existing track", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "original.pdf");
      });

      act(() => {
        store.startTracking("track-1", "doc-2", "replacement.pdf");
      });

      const track = store.getTrack("track-1");
      expect(track?.document_name).toBe("original.pdf");
    });

    it("should initialize all 6 stages in pending state", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const track = store.getTrack("track-1");
      expect(track?.progress.stages).toHaveLength(6);

      const expectedStages: IngestionStage[] = [
        "preprocessing",
        "chunking",
        "extracting",
        "merging",
        "embedding",
        "indexing",
      ];

      track?.progress.stages.forEach((stage, i) => {
        expect(stage.stage).toBe(expectedStages[i]);
        expect(stage.status).toBe("pending");
        expect(stage.progress).toBe(0);
      });
    });
  });

  describe("clearTrack", () => {
    it("should remove specific track", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test1.pdf");
        store.startTracking("track-2", "doc-2", "test2.pdf");
      });

      act(() => {
        store.clearTrack("track-1");
      });

      expect(store.getTrack("track-1")).toBeUndefined();
      expect(store.getTrack("track-2")).toBeDefined();
    });

    it("should handle clearing non-existent track gracefully", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.clearTrack("non-existent");
      });

      // Should not throw
      expect(store.getActiveTracks()).toHaveLength(0);
    });
  });

  describe("clearAllTracks", () => {
    it("should remove all tracks", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test1.pdf");
        store.startTracking("track-2", "doc-2", "test2.pdf");
        store.startTracking("track-3", "doc-3", "test3.pdf");
      });

      expect(store.getActiveTracks()).toHaveLength(3);

      act(() => {
        store.clearAllTracks();
      });

      expect(store.getActiveTracks()).toHaveLength(0);
    });
  });

  describe("getActiveTracks", () => {
    it("should return empty array when no tracks", () => {
      const store = useIngestionStore.getState();
      expect(store.getActiveTracks()).toEqual([]);
    });

    it("should return all active tracks", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test1.pdf");
        store.startTracking("track-2", "doc-2", "test2.pdf");
      });

      const tracks = store.getActiveTracks();
      expect(tracks).toHaveLength(2);
      expect(tracks.map((t) => t.track_id)).toContain("track-1");
      expect(tracks.map((t) => t.track_id)).toContain("track-2");
    });
  });
});

// ============================================================================
// Message Processing Tests
// ============================================================================

describe("Message Processing", () => {
  describe("ingestion_started", () => {
    it("should update existing track to preprocessing", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: IngestionStartedEvent = {
        type: "ingestion_started",
        track_id: "track-1",
        document_id: "doc-1",
        document_name: "test.pdf",
        started_at: new Date().toISOString(),
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      expect(track?.status).toBe("preprocessing");
    });

    it("should create track if not existing", () => {
      const store = useIngestionStore.getState();

      const event: IngestionStartedEvent = {
        type: "ingestion_started",
        track_id: "track-new",
        document_id: "doc-new",
        document_name: "new.pdf",
        started_at: new Date().toISOString(),
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-new");
      expect(track).toBeDefined();
      expect(track?.status).toBe("preprocessing");
    });
  });

  describe("stage_started", () => {
    it("should update track status to stage", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: StageStartedEvent = {
        type: "stage_started",
        track_id: "track-1",
        stage: "chunking",
        started_at: new Date().toISOString(),
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      expect(track?.status).toBe("chunking");
      expect(track?.progress.current_stage).toBe("chunking");
    });

    it("should mark stage as running", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: StageStartedEvent = {
        type: "stage_started",
        track_id: "track-1",
        stage: "extracting",
        started_at: new Date().toISOString(),
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      const extractingStage = track?.progress.stages.find(
        (s) => s.stage === "extracting",
      );
      expect(extractingStage?.status).toBe("running");
    });
  });

  describe("stage_progress", () => {
    it("should update stage progress percentage", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: StageProgressEvent = {
        type: "stage_progress",
        track_id: "track-1",
        stage: "extracting",
        progress: 50,
        current_item: 5,
        total_items: 10,
        message: "Extracting entities from chunk 5/10",
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      const extractingStage = track?.progress.stages.find(
        (s) => s.stage === "extracting",
      );
      expect(extractingStage?.progress).toBe(50);
      expect(extractingStage?.completed_items).toBe(5);
      expect(extractingStage?.total_items).toBe(10);
    });

    it("should update latest message", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: StageProgressEvent = {
        type: "stage_progress",
        track_id: "track-1",
        stage: "chunking",
        progress: 30,
        message: "Processing page 3 of 10",
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      expect(track?.progress.latest_message).toBe("Processing page 3 of 10");
    });

    it("should calculate overall progress with weights", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      // Complete preprocessing (5%) and chunking (10%)
      const completed1: StageCompletedEvent = {
        type: "stage_completed",
        track_id: "track-1",
        stage: "preprocessing",
        completed_at: new Date().toISOString(),
        duration_ms: 1000,
      };

      const completed2: StageCompletedEvent = {
        type: "stage_completed",
        track_id: "track-1",
        stage: "chunking",
        completed_at: new Date().toISOString(),
        duration_ms: 2000,
      };

      act(() => {
        store.updateFromMessage(completed1);
        store.updateFromMessage(completed2);
      });

      const track = store.getTrack("track-1");
      // Preprocessing (5%) + Chunking (10%) = 15% complete
      expect(track?.overall_progress).toBe(15);
    });
  });

  describe("stage_completed", () => {
    it("should mark stage as completed with 100% progress", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: StageCompletedEvent = {
        type: "stage_completed",
        track_id: "track-1",
        stage: "preprocessing",
        completed_at: new Date().toISOString(),
        duration_ms: 1500,
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      const preprocessingStage = track?.progress.stages.find(
        (s) => s.stage === "preprocessing",
      );
      expect(preprocessingStage?.status).toBe("completed");
      expect(preprocessingStage?.progress).toBe(100);
    });

    it("should store duration_ms", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: StageCompletedEvent = {
        type: "stage_completed",
        track_id: "track-1",
        stage: "embedding",
        completed_at: new Date().toISOString(),
        duration_ms: 45000,
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      const embeddingStage = track?.progress.stages.find(
        (s) => s.stage === "embedding",
      );
      expect(embeddingStage?.duration_ms).toBe(45000);
    });

    it("should format result message", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: StageCompletedEvent = {
        type: "stage_completed",
        track_id: "track-1",
        stage: "extracting",
        completed_at: new Date().toISOString(),
        duration_ms: 30000,
        result: {
          entities_extracted: 42,
          relationships_created: 18,
        },
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      const extractingStage = track?.progress.stages.find(
        (s) => s.stage === "extracting",
      );
      expect(extractingStage?.message).toContain("42 entities");
      expect(extractingStage?.message).toContain("18 relationships");
    });
  });

  describe("ingestion_completed", () => {
    it("should mark track as completed", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: IngestionCompletedEvent = {
        type: "ingestion_completed",
        track_id: "track-1",
        document_id: "doc-1",
        completed_at: new Date().toISOString(),
        total_duration_ms: 120000,
        summary: {
          chunks: 25,
          entities: 100,
          relationships: 45,
          total_cost_usd: 0.15,
        },
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      expect(track?.status).toBe("completed");
      expect(track?.overall_progress).toBe(100);
    });

    it("should add to completed jobs", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: IngestionCompletedEvent = {
        type: "ingestion_completed",
        track_id: "track-1",
        document_id: "doc-1",
        completed_at: new Date().toISOString(),
        total_duration_ms: 120000,
        summary: {
          chunks: 25,
          entities: 100,
          relationships: 45,
          total_cost_usd: 0.15,
        },
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const completedJobs = useIngestionStore.getState().completedJobs;
      expect(completedJobs).toHaveLength(1);
      expect(completedJobs[0].chunks).toBe(25);
      expect(completedJobs[0].entities).toBe(100);
    });

    it("should limit completed jobs to 20", () => {
      const store = useIngestionStore.getState();

      // Add 25 completed jobs
      for (let i = 0; i < 25; i++) {
        act(() => {
          store.startTracking(`track-${i}`, `doc-${i}`, `test${i}.pdf`);
        });

        const event: IngestionCompletedEvent = {
          type: "ingestion_completed",
          track_id: `track-${i}`,
          document_id: `doc-${i}`,
          completed_at: new Date().toISOString(),
          total_duration_ms: 1000,
          summary: {
            chunks: i,
            entities: i,
            relationships: i,
            total_cost_usd: 0.01,
          },
        };

        act(() => {
          store.updateFromMessage(event);
        });
      }

      const completedJobs = useIngestionStore.getState().completedJobs;
      expect(completedJobs).toHaveLength(20);
      // Should keep most recent
      expect(completedJobs[19].chunks).toBe(24);
    });
  });

  describe("ingestion_failed", () => {
    it("should mark track as failed", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: IngestionFailedEvent = {
        type: "ingestion_failed",
        track_id: "track-1",
        document_id: "doc-1",
        stage: "extracting",
        failed_at: new Date().toISOString(),
        error: {
          code: "LLM_TIMEOUT",
          message: "LLM request timed out",
          recoverable: true,
        },
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      expect(track?.status).toBe("failed");
    });

    it("should add to failed jobs map", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: IngestionFailedEvent = {
        type: "ingestion_failed",
        track_id: "track-1",
        document_id: "doc-1",
        stage: "embedding",
        failed_at: new Date().toISOString(),
        error: {
          code: "EMBEDDING_ERROR",
          message: "Embedding service unavailable",
          recoverable: true,
        },
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const failedJobs = useIngestionStore.getState().failedJobs;
      expect(failedJobs.has("track-1")).toBe(true);
      expect(failedJobs.get("track-1")?.code).toBe("EMBEDDING_ERROR");
    });

    it("should mark specific stage as failed", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: IngestionFailedEvent = {
        type: "ingestion_failed",
        track_id: "track-1",
        document_id: "doc-1",
        stage: "indexing",
        failed_at: new Date().toISOString(),
        error: {
          code: "INDEX_ERROR",
          message: "Failed to index document",
          recoverable: false,
        },
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      const indexingStage = track?.progress.stages.find(
        (s) => s.stage === "indexing",
      );
      expect(indexingStage?.status).toBe("failed");
    });
  });

  describe("cost_update", () => {
    it("should update latest message with cost", () => {
      const store = useIngestionStore.getState();

      act(() => {
        store.startTracking("track-1", "doc-1", "test.pdf");
      });

      const event: CostUpdateEvent = {
        type: "cost_update",
        track_id: "track-1",
        stage: "extracting",
        operation: "entity_extraction",
        cost_usd: 0.05,
        cumulative_cost_usd: 0.15,
        tokens_used: {
          input: 1000,
          output: 500,
        },
      };

      act(() => {
        store.updateFromMessage(event);
      });

      const track = store.getTrack("track-1");
      expect(track?.progress.latest_message).toContain("$0.1500");
    });
  });
});

// ============================================================================
// WebSocket Status Tests
// ============================================================================

describe("WebSocket Status", () => {
  it("should set connected status", () => {
    const store = useIngestionStore.getState();

    expect(store.wsConnected).toBe(false);

    act(() => {
      store.setWsConnected(true);
    });

    expect(useIngestionStore.getState().wsConnected).toBe(true);
    expect(useIngestionStore.getState().wsReconnecting).toBe(false);
  });

  it("should set reconnecting status", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.setWsReconnecting(true);
    });

    expect(useIngestionStore.getState().wsReconnecting).toBe(true);
  });

  it("should clear reconnecting when connected", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.setWsReconnecting(true);
    });

    act(() => {
      store.setWsConnected(true);
    });

    expect(useIngestionStore.getState().wsConnected).toBe(true);
    expect(useIngestionStore.getState().wsReconnecting).toBe(false);
  });
});

// ============================================================================
// Completed Jobs Tests
// ============================================================================

describe("Completed Jobs", () => {
  it("should add completed job", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.addCompletedJob({
        document_id: "doc-1",
        track_id: "track-1",
        chunks: 10,
        entities: 50,
        relationships: 25,
        duration_ms: 5000,
      });
    });

    expect(useIngestionStore.getState().completedJobs).toHaveLength(1);
  });

  it("should clear completed jobs", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.addCompletedJob({
        document_id: "doc-1",
        track_id: "track-1",
        chunks: 10,
        entities: 50,
        relationships: 25,
        duration_ms: 5000,
      });
      store.addCompletedJob({
        document_id: "doc-2",
        track_id: "track-2",
        chunks: 20,
        entities: 100,
        relationships: 50,
        duration_ms: 10000,
      });
    });

    act(() => {
      store.clearCompletedJobs();
    });

    expect(useIngestionStore.getState().completedJobs).toHaveLength(0);
  });
});

// ============================================================================
// Failed Jobs Tests
// ============================================================================

describe("Failed Jobs", () => {
  it("should add failed job", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.addFailedJob("track-1", {
        code: "ERROR_CODE",
        message: "Error message",
        stage: "extracting",
        reason: "Test reason",
        suggestion: "Try again",
        recoverable: true,
      });
    });

    expect(useIngestionStore.getState().failedJobs.has("track-1")).toBe(true);
  });

  it("should clear specific failed job", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.addFailedJob("track-1", {
        code: "ERROR_1",
        message: "Error 1",
        stage: "chunking",
        reason: "Reason 1",
        suggestion: "Retry chunk",
        recoverable: true,
      });
      store.addFailedJob("track-2", {
        code: "ERROR_2",
        message: "Error 2",
        stage: "embedding",
        reason: "Reason 2",
        suggestion: "Contact support",
        recoverable: false,
      });
    });

    act(() => {
      store.clearFailedJob("track-1");
    });

    expect(useIngestionStore.getState().failedJobs.has("track-1")).toBe(false);
    expect(useIngestionStore.getState().failedJobs.has("track-2")).toBe(true);
  });

  it("should clear all failed jobs", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.addFailedJob("track-1", {
        code: "ERROR_1",
        message: "Error 1",
        stage: "preprocessing",
        reason: "Reason 1",
        suggestion: "Retry preprocessing",
        recoverable: true,
      });
      store.addFailedJob("track-2", {
        code: "ERROR_2",
        message: "Error 2",
        stage: "indexing",
        reason: "Reason 2",
        suggestion: "Contact admin",
        recoverable: false,
      });
    });

    act(() => {
      store.clearAllFailedJobs();
    });

    expect(useIngestionStore.getState().failedJobs.size).toBe(0);
  });
});

// ============================================================================
// StopTracking Tests
// ============================================================================

describe("stopTracking", () => {
  it("should remove completed track", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.startTracking("track-1", "doc-1", "test.pdf");
    });

    // Complete the track
    const event: IngestionCompletedEvent = {
      type: "ingestion_completed",
      track_id: "track-1",
      document_id: "doc-1",
      completed_at: new Date().toISOString(),
      total_duration_ms: 1000,
      summary: {
        chunks: 10,
        entities: 50,
        relationships: 25,
        total_cost_usd: 0.05,
      },
    };

    act(() => {
      store.updateFromMessage(event);
    });

    act(() => {
      store.stopTracking("track-1");
    });

    expect(store.getTrack("track-1")).toBeUndefined();
  });

  it("should remove failed track", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.startTracking("track-1", "doc-1", "test.pdf");
    });

    // Fail the track
    const event: IngestionFailedEvent = {
      type: "ingestion_failed",
      track_id: "track-1",
      document_id: "doc-1",
      stage: "extracting",
      failed_at: new Date().toISOString(),
      error: {
        code: "ERROR",
        message: "Error",
        recoverable: true,
      },
    };

    act(() => {
      store.updateFromMessage(event);
    });

    act(() => {
      store.stopTracking("track-1");
    });

    expect(store.getTrack("track-1")).toBeUndefined();
  });

  it("should NOT remove in-progress track", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.startTracking("track-1", "doc-1", "test.pdf");
    });

    // Start ingestion (pending -> preprocessing)
    const event: IngestionStartedEvent = {
      type: "ingestion_started",
      track_id: "track-1",
      document_id: "doc-1",
      document_name: "test.pdf",
      started_at: new Date().toISOString(),
    };

    act(() => {
      store.updateFromMessage(event);
    });

    act(() => {
      store.stopTracking("track-1");
    });

    // Should still exist because it's in preprocessing (not completed/failed)
    expect(store.getTrack("track-1")).toBeDefined();
  });
});

// ============================================================================
// Edge Cases
// ============================================================================

describe("Edge Cases", () => {
  it("should handle messages for non-existent tracks gracefully", () => {
    const store = useIngestionStore.getState();

    const event: StageProgressEvent = {
      type: "stage_progress",
      track_id: "non-existent",
      stage: "chunking",
      progress: 50,
    };

    // Should not throw
    act(() => {
      store.updateFromMessage(event);
    });

    expect(store.getTrack("non-existent")).toBeUndefined();
  });

  it("should handle unknown message types", () => {
    const store = useIngestionStore.getState();

    const event = {
      type: "unknown_type",
      track_id: "track-1",
    } as unknown;

    // Should not throw
    act(() => {
      store.updateFromMessage(event as IngestionStartedEvent);
    });

    // State should remain unchanged
    expect(store.getActiveTracks()).toHaveLength(0);
  });

  it("should handle rapid progress updates", () => {
    const store = useIngestionStore.getState();

    act(() => {
      store.startTracking("track-1", "doc-1", "test.pdf");
    });

    // Simulate rapid updates
    for (let i = 0; i <= 100; i += 10) {
      const event: StageProgressEvent = {
        type: "stage_progress",
        track_id: "track-1",
        stage: "extracting",
        progress: i,
      };

      act(() => {
        store.updateFromMessage(event);
      });
    }

    const track = store.getTrack("track-1");
    const extractingStage = track?.progress.stages.find(
      (s) => s.stage === "extracting",
    );
    expect(extractingStage?.progress).toBe(100);
  });

  it("should handle concurrent track updates", () => {
    const store = useIngestionStore.getState();

    // Start multiple tracks
    act(() => {
      store.startTracking("track-1", "doc-1", "test1.pdf");
      store.startTracking("track-2", "doc-2", "test2.pdf");
      store.startTracking("track-3", "doc-3", "test3.pdf");
    });

    // Update all concurrently
    act(() => {
      store.updateFromMessage({
        type: "stage_progress",
        track_id: "track-1",
        stage: "chunking",
        progress: 50,
      } as StageProgressEvent);

      store.updateFromMessage({
        type: "stage_progress",
        track_id: "track-2",
        stage: "extracting",
        progress: 75,
      } as StageProgressEvent);

      store.updateFromMessage({
        type: "stage_progress",
        track_id: "track-3",
        stage: "embedding",
        progress: 25,
      } as StageProgressEvent);
    });

    expect(
      store
        .getTrack("track-1")
        ?.progress.stages.find((s) => s.stage === "chunking")?.progress,
    ).toBe(50);
    expect(
      store
        .getTrack("track-2")
        ?.progress.stages.find((s) => s.stage === "extracting")?.progress,
    ).toBe(75);
    expect(
      store
        .getTrack("track-3")
        ?.progress.stages.find((s) => s.stage === "embedding")?.progress,
    ).toBe(25);
  });
});
