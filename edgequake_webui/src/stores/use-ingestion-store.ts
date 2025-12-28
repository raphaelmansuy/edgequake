/**
 * Ingestion Progress Store
 *
 * Zustand store for managing real-time ingestion progress state.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 */

import type { CostUpdateEvent } from "@/types/cost";
import type {
  IngestionCompletedEvent,
  IngestionError,
  IngestionFailedEvent,
  IngestionProgress,
  IngestionResult,
  IngestionStage,
  IngestionStartedEvent,
  StageCompletedEvent,
  StageProgress,
  StageProgressEvent,
  StageStartedEvent,
  WebSocketProgressMessage,
} from "@/types/ingestion";
import { create } from "zustand";
import { devtools } from "zustand/middleware";

// ============================================================================
// Store Types
// ============================================================================

interface IngestionState {
  // Active ingestion tracks
  tracks: Map<string, IngestionProgress>;

  // WebSocket connection status
  wsConnected: boolean;
  wsReconnecting: boolean;

  // Completed jobs (recent history)
  completedJobs: IngestionResult[];

  // Failed jobs (for retry)
  failedJobs: Map<string, IngestionError>;
}

interface IngestionActions {
  // Track management
  startTracking: (
    trackId: string,
    documentId: string,
    documentName: string
  ) => void;
  updateFromMessage: (
    message: WebSocketProgressMessage | CostUpdateEvent
  ) => void;
  stopTracking: (trackId: string) => void;
  clearTrack: (trackId: string) => void;
  clearAllTracks: () => void;

  // WebSocket status
  setWsConnected: (connected: boolean) => void;
  setWsReconnecting: (reconnecting: boolean) => void;

  // Completed jobs
  addCompletedJob: (result: IngestionResult) => void;
  clearCompletedJobs: () => void;

  // Failed jobs
  addFailedJob: (trackId: string, error: IngestionError) => void;
  clearFailedJob: (trackId: string) => void;
  clearAllFailedJobs: () => void;

  // Getters
  getTrack: (trackId: string) => IngestionProgress | undefined;
  getActiveTracks: () => IngestionProgress[];
}

type IngestionStore = IngestionState & IngestionActions;

// ============================================================================
// Helper Functions
// ============================================================================

function createInitialStages(): StageProgress[] {
  const stages: IngestionStage[] = [
    "preprocessing",
    "chunking",
    "extracting",
    "merging",
    "embedding",
    "indexing",
  ];

  return stages.map((stage) => ({
    stage,
    status: "pending",
    progress: 0,
    total_items: 0,
    completed_items: 0,
  }));
}

function createInitialProgress(
  trackId: string,
  documentId: string,
  documentName: string
): IngestionProgress {
  return {
    track_id: trackId,
    document_id: documentId,
    document_name: documentName,
    status: "pending",
    overall_progress: 0,
    progress: {
      current_stage: "pending",
      completion_percentage: 0,
      latest_message: "Waiting to start...",
      stages: createInitialStages(),
    },
    started_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
  };
}

function handleIngestionStarted(
  state: IngestionState,
  event: IngestionStartedEvent
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);

  const existing = tracks.get(event.track_id);
  if (existing) {
    existing.status = "preprocessing";
    existing.started_at = event.started_at;
    existing.updated_at = event.started_at;
    existing.progress.latest_message = "Ingestion started...";
  } else {
    tracks.set(event.track_id, {
      ...createInitialProgress(
        event.track_id,
        event.document_id,
        event.document_name
      ),
      status: "preprocessing",
      started_at: event.started_at,
    });
  }

  return tracks;
}

function handleStageStarted(
  state: IngestionState,
  event: StageStartedEvent
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.track_id);

  if (track) {
    track.status = event.stage;
    track.updated_at = event.started_at;
    track.progress.current_stage = event.stage;
    track.progress.latest_message = `Starting ${event.stage}...`;

    // Update stage status
    const stageIndex = track.progress.stages.findIndex(
      (s) => s.stage === event.stage
    );
    if (stageIndex >= 0) {
      track.progress.stages[stageIndex].status = "running";
      track.progress.stages[stageIndex].started_at = event.started_at;
    }
  }

  return tracks;
}

function handleStageProgress(
  state: IngestionState,
  event: StageProgressEvent
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.track_id);

  if (track) {
    track.updated_at = new Date().toISOString();

    // Update stage progress
    const stageIndex = track.progress.stages.findIndex(
      (s) => s.stage === event.stage
    );
    if (stageIndex >= 0) {
      track.progress.stages[stageIndex].progress = event.progress;
      if (event.current_item !== undefined) {
        track.progress.stages[stageIndex].completed_items = event.current_item;
      }
      if (event.total_items !== undefined) {
        track.progress.stages[stageIndex].total_items = event.total_items;
      }
    }

    // Update overall progress
    track.progress.completion_percentage = calculateOverallProgress(
      track.progress.stages
    );
    track.overall_progress = track.progress.completion_percentage;

    if (event.message) {
      track.progress.latest_message = event.message;
    }
  }

  return tracks;
}

function handleStageCompleted(
  state: IngestionState,
  event: StageCompletedEvent
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.track_id);

  if (track) {
    track.updated_at = event.completed_at;

    // Update stage status
    const stageIndex = track.progress.stages.findIndex(
      (s) => s.stage === event.stage
    );
    if (stageIndex >= 0) {
      track.progress.stages[stageIndex].status = "completed";
      track.progress.stages[stageIndex].progress = 100;
      track.progress.stages[stageIndex].completed_at = event.completed_at;
      track.progress.stages[stageIndex].duration_ms = event.duration_ms;

      if (event.result) {
        track.progress.stages[stageIndex].message = formatStageResult(
          event.result
        );
      }
    }

    // Update overall progress
    track.progress.completion_percentage = calculateOverallProgress(
      track.progress.stages
    );
    track.overall_progress = track.progress.completion_percentage;
    track.progress.latest_message = `Completed ${event.stage}`;
  }

  return tracks;
}

function handleIngestionCompleted(
  state: IngestionState,
  event: IngestionCompletedEvent
): { tracks: Map<string, IngestionProgress>; completedJob: IngestionResult } {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.track_id);

  if (track) {
    track.status = "completed";
    track.overall_progress = 100;
    track.completed_at = event.completed_at;
    track.updated_at = event.completed_at;
    track.progress.completion_percentage = 100;
    track.progress.latest_message = "Ingestion completed successfully";

    // Mark all stages as completed
    track.progress.stages.forEach((stage) => {
      if (stage.status !== "failed") {
        stage.status = "completed";
        stage.progress = 100;
      }
    });
  }

  const completedJob: IngestionResult = {
    document_id: event.document_id,
    track_id: event.track_id,
    chunks: event.summary.chunks,
    entities: event.summary.entities,
    relationships: event.summary.relationships,
    duration_ms: event.total_duration_ms,
  };

  return { tracks, completedJob };
}

function handleIngestionFailed(
  state: IngestionState,
  event: IngestionFailedEvent
): { tracks: Map<string, IngestionProgress>; error: IngestionError } {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.track_id);

  if (track) {
    track.status = "failed";
    track.updated_at = event.failed_at;
    track.progress.latest_message = `Failed: ${event.error.message}`;

    // Mark the failed stage
    const stageIndex = track.progress.stages.findIndex(
      (s) => s.stage === event.stage
    );
    if (stageIndex >= 0) {
      track.progress.stages[stageIndex].status = "failed";
    }
  }

  const error: IngestionError = {
    code: event.error.code,
    message: event.error.message,
    stage: event.stage,
    reason: event.error.message,
    suggestion: event.error.recoverable
      ? "You can retry this operation."
      : "Please check the logs for more details.",
    recoverable: event.error.recoverable,
  };

  return { tracks, error };
}

function calculateOverallProgress(stages: StageProgress[]): number {
  const weights = {
    preprocessing: 5,
    chunking: 10,
    extracting: 50,
    merging: 15,
    embedding: 15,
    indexing: 5,
  };

  let totalWeight = 0;
  let completedWeight = 0;

  stages.forEach((stage) => {
    const weight = weights[stage.stage as keyof typeof weights] || 10;
    totalWeight += weight;

    if (stage.status === "completed") {
      completedWeight += weight;
    } else if (stage.status === "running") {
      completedWeight += (weight * stage.progress) / 100;
    }
  });

  return totalWeight > 0 ? (completedWeight / totalWeight) * 100 : 0;
}

function formatStageResult(result: {
  chunks_created?: number;
  entities_extracted?: number;
  relationships_created?: number;
}): string {
  const parts = [];
  if (result.chunks_created) parts.push(`${result.chunks_created} chunks`);
  if (result.entities_extracted)
    parts.push(`${result.entities_extracted} entities`);
  if (result.relationships_created)
    parts.push(`${result.relationships_created} relationships`);
  return parts.join(", ");
}

// ============================================================================
// Store Definition
// ============================================================================

export const useIngestionStore = create<IngestionStore>()(
  devtools(
    (set, get) => ({
      // Initial state
      tracks: new Map(),
      wsConnected: false,
      wsReconnecting: false,
      completedJobs: [],
      failedJobs: new Map(),

      // Track management
      startTracking: (trackId, documentId, documentName) => {
        set((state) => {
          const tracks = new Map(state.tracks);
          if (!tracks.has(trackId)) {
            tracks.set(
              trackId,
              createInitialProgress(trackId, documentId, documentName)
            );
          }
          return { tracks };
        });
      },

      updateFromMessage: (message) => {
        set((state) => {
          switch (message.type) {
            case "ingestion_started":
              return {
                tracks: handleIngestionStarted(
                  state,
                  message as IngestionStartedEvent
                ),
              };

            case "stage_started":
              return {
                tracks: handleStageStarted(state, message as StageStartedEvent),
              };

            case "stage_progress":
              return {
                tracks: handleStageProgress(
                  state,
                  message as StageProgressEvent
                ),
              };

            case "stage_completed":
              return {
                tracks: handleStageCompleted(
                  state,
                  message as StageCompletedEvent
                ),
              };

            case "ingestion_completed": {
              const { tracks, completedJob } = handleIngestionCompleted(
                state,
                message as IngestionCompletedEvent
              );
              return {
                tracks,
                completedJobs: [
                  ...state.completedJobs.slice(-19),
                  completedJob,
                ],
              };
            }

            case "ingestion_failed": {
              const { tracks, error } = handleIngestionFailed(
                state,
                message as IngestionFailedEvent
              );
              const failedJobs = new Map(state.failedJobs);
              failedJobs.set((message as IngestionFailedEvent).track_id, error);
              return { tracks, failedJobs };
            }

            case "cost_update": {
              // Handle cost updates (integrate with cost store)
              const tracks = new Map(state.tracks);
              const costEvent = message as CostUpdateEvent;
              const track = tracks.get(costEvent.track_id);
              if (track) {
                track.updated_at = new Date().toISOString();
                track.progress.latest_message = `Cost: $${costEvent.cumulative_cost_usd.toFixed(
                  4
                )}`;
              }
              return { tracks };
            }

            default:
              return state;
          }
        });
      },

      stopTracking: (trackId) => {
        set((state) => {
          const tracks = new Map(state.tracks);
          const track = tracks.get(trackId);
          if (
            track &&
            (track.status === "completed" || track.status === "failed")
          ) {
            tracks.delete(trackId);
          }
          return { tracks };
        });
      },

      clearTrack: (trackId) => {
        set((state) => {
          const tracks = new Map(state.tracks);
          tracks.delete(trackId);
          return { tracks };
        });
      },

      clearAllTracks: () => {
        set({ tracks: new Map() });
      },

      // WebSocket status
      setWsConnected: (connected) => {
        set({ wsConnected: connected, wsReconnecting: false });
      },

      setWsReconnecting: (reconnecting) => {
        set({ wsReconnecting: reconnecting });
      },

      // Completed jobs
      addCompletedJob: (result) => {
        set((state) => ({
          completedJobs: [...state.completedJobs.slice(-19), result],
        }));
      },

      clearCompletedJobs: () => {
        set({ completedJobs: [] });
      },

      // Failed jobs
      addFailedJob: (trackId, error) => {
        set((state) => {
          const failedJobs = new Map(state.failedJobs);
          failedJobs.set(trackId, error);
          return { failedJobs };
        });
      },

      clearFailedJob: (trackId) => {
        set((state) => {
          const failedJobs = new Map(state.failedJobs);
          failedJobs.delete(trackId);
          return { failedJobs };
        });
      },

      clearAllFailedJobs: () => {
        set({ failedJobs: new Map() });
      },

      // Getters
      getTrack: (trackId) => {
        return get().tracks.get(trackId);
      },

      getActiveTracks: () => {
        return Array.from(get().tracks.values());
      },
    }),
    { name: "ingestion-store" }
  )
);
