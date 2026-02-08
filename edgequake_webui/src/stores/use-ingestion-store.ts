/**
 * Ingestion Progress Store
 *
 * Zustand store for managing real-time ingestion progress state.
 * Based on WebUI Specification Document WEBUI-005 (14-webui-websocket-progress.md)
 *
 * @implements UC0001 - Upload Document (progress tracking)
 * @implements UC0007 - View Ingestion Progress
 * @implements UC0008 - Cancel Ingestion
 * @implements FEAT0001 - Document Ingestion Pipeline (frontend visibility)
 * @implements FEAT0602 - Real-time Progress Updates (WebSocket)
 *
 * @enforces BR0302 - Document size limits (via backend, tracked here)
 * @enforces BR0303 - Cost tracking per request (via CostUpdateEvent)
 *
 * @description
 * This Zustand store manages:
 * - Active ingestion tracks (Map by trackId)
 * - WebSocket connection status
 * - Completed and failed job history
 * - Real-time progress updates from backend
 *
 * @see WEBUI-005 for WebSocket protocol specification
 * @see useBackendStore for WebSocket connection management
 */

import type { CostUpdateEvent } from "@/types/cost";
import type {
    ChunkFailureEvent,
    ChunkProgressEvent,
    IngestionCompletedEvent,
    IngestionError,
    IngestionFailedEvent,
    IngestionProgress,
    IngestionResult,
    IngestionStage,
    IngestionStartedEvent,
    PdfPageProgressEvent,
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
  wsMaxReconnectsReached: boolean;

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
    documentName: string,
  ) => void;
  updateFromMessage: (
    message: WebSocketProgressMessage | CostUpdateEvent,
  ) => void;
  stopTracking: (trackId: string) => void;
  clearTrack: (trackId: string) => void;
  clearAllTracks: () => void;

  // WebSocket status
  setWsConnected: (connected: boolean) => void;
  setWsReconnecting: (reconnecting: boolean) => void;
  setWsMaxReconnectsReached: (reached: boolean) => void;

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
  documentName: string,
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
  event: IngestionStartedEvent,
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
        event.document_name,
      ),
      status: "preprocessing",
      started_at: event.started_at,
    });
  }

  return tracks;
}

function handleStageStarted(
  state: IngestionState,
  event: StageStartedEvent,
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
      (s) => s.stage === event.stage,
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
  event: StageProgressEvent,
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.track_id);

  if (track) {
    track.updated_at = new Date().toISOString();

    // Update stage progress
    const stageIndex = track.progress.stages.findIndex(
      (s) => s.stage === event.stage,
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
      track.progress.stages,
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
  event: StageCompletedEvent,
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.track_id);

  if (track) {
    track.updated_at = event.completed_at;

    // Update stage status
    const stageIndex = track.progress.stages.findIndex(
      (s) => s.stage === event.stage,
    );
    if (stageIndex >= 0) {
      track.progress.stages[stageIndex].status = "completed";
      track.progress.stages[stageIndex].progress = 100;
      track.progress.stages[stageIndex].completed_at = event.completed_at;
      track.progress.stages[stageIndex].duration_ms = event.duration_ms;

      if (event.result) {
        track.progress.stages[stageIndex].message = formatStageResult(
          event.result,
        );
      }
    }

    // Update overall progress
    track.progress.completion_percentage = calculateOverallProgress(
      track.progress.stages,
    );
    track.overall_progress = track.progress.completion_percentage;
    track.progress.latest_message = `Completed ${event.stage}`;
  }

  return tracks;
}

function handleIngestionCompleted(
  state: IngestionState,
  event: IngestionCompletedEvent,
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
  event: IngestionFailedEvent,
): { tracks: Map<string, IngestionProgress>; error: IngestionError } {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.track_id);

  if (track) {
    track.status = "failed";
    track.updated_at = event.failed_at;
    track.progress.latest_message = `Failed: ${event.error.message}`;

    // Mark the failed stage
    const stageIndex = track.progress.stages.findIndex(
      (s) => s.stage === event.stage,
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

/**
 * Handle PDF page progress event.
 *
 * @implements OODA-06: PDF page-by-page progress tracking
 *
 * WHY: Large PDFs (30+ pages) take significant time. This provides
 * page-level granularity so users see continuous progress during
 * PDF→Markdown conversion phase.
 */
function handlePdfPageProgress(
  state: IngestionState,
  event: PdfPageProgressEvent,
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.data.task_id);

  if (track) {
    track.updated_at = new Date().toISOString();

    // Initialize or update PDF progress
    track.progress.pdf_progress = {
      current_page: event.data.current_page,
      total_pages: event.data.total_pages,
      progress: Math.round(event.data.progress * 100),
      phase: "extraction",
    };

    // Update latest message with page info
    if (event.data.current_page > 0) {
      track.progress.latest_message = `Converting PDF to Markdown: page ${event.data.current_page}/${event.data.total_pages} (${Math.round(event.data.progress * 100)}%)`;
    } else {
      track.progress.latest_message = `Starting PDF extraction (${event.data.total_pages} pages)...`;
    }

    // Update converting stage if it exists
    const convertingStageIndex = track.progress.stages.findIndex(
      (s) => s.stage === "converting",
    );
    if (convertingStageIndex >= 0) {
      track.progress.stages[convertingStageIndex].progress = Math.round(
        event.data.progress * 100,
      );
      track.progress.stages[convertingStageIndex].completed_items =
        event.data.current_page;
      track.progress.stages[convertingStageIndex].total_items =
        event.data.total_pages;
    }

    // Recalculate overall progress
    track.overall_progress = calculateOverallProgress(track.progress.stages);
  }

  return tracks;
}

/**
 * Handle chunk extraction progress event.
 *
 * @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
 *
 * WHY: Entity extraction processes chunks in parallel. This provides
 * chunk-level granularity showing real-time progress through the
 * map-reduce extraction phase.
 */
function handleChunkProgress(
  state: IngestionState,
  event: ChunkProgressEvent,
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.data.task_id);

  if (track) {
    track.updated_at = new Date().toISOString();

    // Calculate chunk progress percentage
    const progress =
      event.data.total_chunks > 0
        ? Math.round(
            ((event.data.chunk_index + 1) / event.data.total_chunks) * 100,
          )
        : 0;

    // Initialize or update chunk progress
    track.progress.chunk_progress = {
      current_chunk: event.data.chunk_index + 1, // Convert to 1-indexed
      total_chunks: event.data.total_chunks,
      progress,
      current_chunk_preview: event.data.chunk_preview,
      eta_seconds: event.data.eta_seconds,
      cumulative_cost: event.data.cost_usd,
      failed_chunks: track.progress.chunk_progress?.failed_chunks ?? 0,
    };

    // Update latest message with chunk info
    const etaMsg = event.data.eta_seconds
      ? ` (ETA: ${Math.round(event.data.eta_seconds)}s)`
      : "";
    track.progress.latest_message = `Extracting entities: chunk ${event.data.chunk_index + 1}/${event.data.total_chunks}${etaMsg}`;

    // Update extracting stage progress
    const extractingStageIndex = track.progress.stages.findIndex(
      (s) => s.stage === "extracting",
    );
    if (extractingStageIndex >= 0) {
      track.progress.stages[extractingStageIndex].progress = progress;
      track.progress.stages[extractingStageIndex].completed_items =
        event.data.chunk_index + 1;
      track.progress.stages[extractingStageIndex].total_items =
        event.data.total_chunks;
    }

    // Recalculate overall progress
    track.overall_progress = calculateOverallProgress(track.progress.stages);
  }

  return tracks;
}

/**
 * Handle chunk extraction failure event.
 *
 * @implements SPEC-003: Chunk-level resilience with failure visibility
 *
 * WHY: When using resilient processing, some chunks may fail while
 * others succeed. This tracks failed chunks for UI display and debugging.
 */
function handleChunkFailure(
  state: IngestionState,
  event: ChunkFailureEvent,
): Map<string, IngestionProgress> {
  const tracks = new Map(state.tracks);
  const track = tracks.get(event.data.task_id);

  if (track && track.progress.chunk_progress) {
    track.updated_at = new Date().toISOString();

    // Increment failed chunks counter
    track.progress.chunk_progress.failed_chunks += 1;

    // Update latest message with failure info
    const timeoutMsg = event.data.was_timeout ? " (timeout)" : "";
    track.progress.latest_message = `Chunk ${event.data.chunk_index + 1} failed${timeoutMsg}: ${event.data.error_message}`;

    // Log warning for debugging
    console.warn(
      `[IngestionStore] Chunk ${event.data.chunk_index + 1}/${event.data.total_chunks} failed for ${track.document_name}:`,
      event.data.error_message,
    );
  }

  return tracks;
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
      wsMaxReconnectsReached: false,
      completedJobs: [],
      failedJobs: new Map(),

      // Track management
      startTracking: (trackId, documentId, documentName) => {
        set((state) => {
          const tracks = new Map(state.tracks);
          if (!tracks.has(trackId)) {
            tracks.set(
              trackId,
              createInitialProgress(trackId, documentId, documentName),
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
                  message as IngestionStartedEvent,
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
                  message as StageProgressEvent,
                ),
              };

            case "stage_completed":
              return {
                tracks: handleStageCompleted(
                  state,
                  message as StageCompletedEvent,
                ),
              };

            case "ingestion_completed": {
              const { tracks, completedJob } = handleIngestionCompleted(
                state,
                message as IngestionCompletedEvent,
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
                message as IngestionFailedEvent,
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
                  4,
                )}`;
              }
              return { tracks };
            }

            case "PdfPageProgress":
              return {
                tracks: handlePdfPageProgress(
                  state,
                  message as PdfPageProgressEvent,
                ),
              };

            case "ChunkProgress":
              return {
                tracks: handleChunkProgress(
                  state,
                  message as ChunkProgressEvent,
                ),
              };

            case "ChunkFailure":
              return {
                tracks: handleChunkFailure(state, message as ChunkFailureEvent),
              };

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

      setWsMaxReconnectsReached: (reached) => {
        set({ wsMaxReconnectsReached: reached });
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
    { name: "ingestion-store" },
  ),
);
