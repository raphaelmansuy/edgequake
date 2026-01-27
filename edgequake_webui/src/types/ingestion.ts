/**
 * @module ingestion-types
 * @description Types for real-time ingestion progress tracking.
 * Based on WebUI Specification Document WEBUI-003 (12-webui-api-integration.md)
 *
 * @implements UC0007 - Monitor document processing progress
 * @implements FEAT0602 - Real-time progress indicators
 * @implements FEAT0625 - Stage-by-stage progress tracking
 *
 * @enforces BR0302 - Progress visible for all active uploads
 * @enforces BR0615 - Stage transitions logged
 *
 * @see {@link specs/WEBUI-003.md} for specification
 */

// ============================================================================
// Ingestion Status Types
// ============================================================================

export type IngestionStage =
  | "pending"
  | "preprocessing"
  | "chunking"
  | "extracting"
  | "gleaning"
  | "merging"
  | "summarizing"
  | "embedding"
  | "indexing";

export type IngestionStatus =
  | IngestionStage
  | "processing"
  | "completed"
  | "failed"
  | "cancelled";

export type StageStatus = "pending" | "running" | "completed" | "failed";

// ============================================================================
// Progress Tracking Types
// ============================================================================

export interface StageProgress {
  stage: IngestionStage;
  status: StageStatus;
  progress: number; // 0-100
  total_items: number;
  completed_items: number;
  started_at?: string;
  completed_at?: string;
  duration_ms?: number;
  message?: string;
}

export interface ProgressDetail {
  current_stage: IngestionStage;
  completion_percentage: number;
  eta_seconds?: number;
  latest_message: string;
  stages: StageProgress[];
}

export interface IngestionProgress {
  track_id: string;
  document_id: string;
  document_name: string;
  status: IngestionStatus;
  overall_progress: number;
  progress: ProgressDetail;
  started_at?: string;
  updated_at?: string;
  completed_at?: string;
}

// ============================================================================
// Error Types
// ============================================================================

export interface IngestionError {
  code: string;
  message: string;
  stage: IngestionStage;
  reason: string;
  suggestion: string;
  recoverable: boolean;
  partial_result?: {
    chunks_processed: number;
    entities_extracted: number;
    relationships_found: number;
  };
}

export interface IngestionResult {
  document_id: string;
  track_id: string;
  chunks: number;
  entities: number;
  relationships: number;
  duration_ms: number;
}

// ============================================================================
// WebSocket Message Types
// ============================================================================

export interface IngestionStartedEvent {
  type: "ingestion_started";
  track_id: string;
  document_id: string;
  document_name: string;
  started_at: string;
  estimated_duration_ms?: number;
}

export interface StageStartedEvent {
  type: "stage_started";
  track_id: string;
  stage: IngestionStage;
  started_at: string;
}

export interface StageProgressEvent {
  type: "stage_progress";
  track_id: string;
  stage: IngestionStage;
  progress: number; // 0-100
  message?: string;
  current_item?: number;
  total_items?: number;
}

export interface StageCompletedEvent {
  type: "stage_completed";
  track_id: string;
  stage: IngestionStage;
  completed_at: string;
  duration_ms: number;
  result?: {
    chunks_created?: number;
    entities_extracted?: number;
    relationships_created?: number;
  };
}

export interface IngestionCompletedEvent {
  type: "ingestion_completed";
  track_id: string;
  document_id: string;
  completed_at: string;
  total_duration_ms: number;
  summary: {
    chunks: number;
    entities: number;
    relationships: number;
    total_cost_usd: number;
  };
}

export interface IngestionFailedEvent {
  type: "ingestion_failed";
  track_id: string;
  document_id?: string;
  stage: IngestionStage;
  error: {
    code: string;
    message: string;
    recoverable: boolean;
    retry_after_ms?: number;
  };
  failed_at: string;
}

export interface HeartbeatEvent {
  type: "heartbeat";
  timestamp: string;
  server_time: string;
}

/**
 * Chunk-level progress event for granular extraction visibility.
 *
 * @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
 *
 * WHY: The real progression of document ingestion is chunks processed
 * vs chunks remaining. This event provides granular visibility into
 * the map-reduce extraction phase where each chunk is processed.
 */
export interface ChunkProgressEvent {
  type: "ChunkProgress";
  data: {
    /** Document being processed */
    document_id: string;
    /** Task tracking ID */
    task_id: string;
    /** Current chunk index (0-based) */
    chunk_index: number;
    /** Total chunks in document */
    total_chunks: number;
    /** Preview of current chunk (first 80 chars) */
    chunk_preview: string;
    /** Time taken for this chunk (milliseconds) */
    time_ms: number;
    /** Estimated time remaining (seconds) */
    eta_seconds: number;
    /** Cumulative input tokens */
    tokens_in: number;
    /** Cumulative output tokens */
    tokens_out: number;
    /** Cumulative cost (USD) */
    cost_usd: number;
  };
}

export type WebSocketProgressMessage =
  | IngestionStartedEvent
  | StageStartedEvent
  | StageProgressEvent
  | StageCompletedEvent
  | IngestionCompletedEvent
  | IngestionFailedEvent
  | HeartbeatEvent
  | ChunkProgressEvent;

// ============================================================================
// Client Command Types
// ============================================================================

export interface SubscribeCommand {
  type: "subscribe";
  track_ids: string[];
}

export interface UnsubscribeCommand {
  type: "unsubscribe";
  track_ids: string[];
}

export interface CancelIngestionCommand {
  type: "cancel";
  track_id: string;
}

export interface PingCommand {
  type: "ping";
  client_time: string;
}

export type ClientCommand =
  | SubscribeCommand
  | UnsubscribeCommand
  | CancelIngestionCommand
  | PingCommand;

// ============================================================================
// API Response Types
// ============================================================================

export interface TrackProgressResponse {
  track_id: string;
  document_id: string;
  document_name: string;
  status: IngestionStatus;
  progress: ProgressDetail;
  started_at: string;
  updated_at: string;
  completed_at?: string;
}
