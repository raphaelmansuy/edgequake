/**
 * @module ingestion-types
 * @description Types for real-time ingestion progress tracking.
 * Based on WebUI Specification Document WEBUI-003 (12-webui-api-integration.md)
 *
 * @implements UC0007 - Monitor document processing progress
 * @implements FEAT0602 - Real-time progress indicators
 * @implements FEAT0625 - Stage-by-stage progress tracking
 * @implements SPEC-002 - Unified Ingestion Pipeline
 *
 * @enforces BR0302 - Progress visible for all active uploads
 * @enforces BR0615 - Stage transitions logged
 *
 * @see {@link specs/WEBUI-003.md} for specification
 * @see {@link specs/002-unify-ingestion-pipeline.md} for unified pipeline spec
 */

// ============================================================================
// Source Type
// ============================================================================

/**
 * Source type for ingestion.
 * Determines which pipeline stages are applicable.
 */
export type SourceType = "pdf" | "markdown" | "text";

// ============================================================================
// Unified Ingestion Stages
// ============================================================================

/**
 * Unified ingestion stage - aligns with backend UnifiedStage enum.
 *
 * Stage Flow:
 * [uploading] → [converting?] → [preprocessing] → [chunking]
 *      ↓              ↓               ↓               ↓
 * [extracting] → [gleaning] → [merging] → [summarizing]
 *      ↓              ↓           ↓            ↓
 * [embedding] → [storing] → [completed/failed]
 *
 * Note: 'converting' stage only applies to PDF sources.
 * Legacy aliases: pending → uploading, indexing → storing
 */
export type IngestionStage =
  | "uploading" // File/content being uploaded
  | "converting" // PDF → Markdown (PDF only)
  | "preprocessing" // Validation, parsing
  | "chunking" // Document splitting
  | "extracting" // Entity/relationship extraction
  | "gleaning" // Second pass extraction
  | "merging" // Graph merge
  | "summarizing" // Description summarization
  | "embedding" // Vector generation
  | "storing" // Persist to storage
  | "completed" // Successfully finished
  | "failed" // Error state
  // Legacy aliases for backward compatibility
  | "pending" // Alias for uploading (legacy)
  | "indexing"; // Alias for storing (legacy)

/**
 * Legacy stage names for backward compatibility.
 * Map to unified stages where possible.
 */
export type LegacyStage = "processing"; // Generic active state

export type IngestionStatus = IngestionStage | LegacyStage | "cancelled";

export type StageStatus =
  | "pending"
  | "running"
  | "completed"
  | "skipped"
  | "failed";

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

/**
 * PDF extraction progress details.
 *
 * @implements OODA-06: PDF page-by-page progress tracking
 */
export interface PdfProgress {
  /** Current page being extracted */
  current_page: number;
  /** Total pages in PDF */
  total_pages: number;
  /** Progress percentage (0-100) */
  progress: number;
  /** Current phase: "start", "extraction", "complete" */
  phase: string;
  /** Last error encountered during extraction */
  last_error?: string;
}

/**
 * Chunk extraction progress details.
 *
 * @implements SPEC-001/Objective-A: Chunk-Level Progress Visibility
 */
export interface ChunkProgress {
  /** Current chunk index (0-based) */
  current_chunk: number;
  /** Total chunks in document */
  total_chunks: number;
  /** Progress percentage (0-100) */
  progress: number;
  /** Preview of current chunk (first 80 chars) */
  current_chunk_preview?: string;
  /** Estimated time remaining (seconds) */
  eta_seconds?: number;
  /** Cumulative cost (USD) */
  cumulative_cost?: number;
  /** Failed chunks count */
  failed_chunks: number;
}

export interface ProgressDetail {
  current_stage: IngestionStage;
  completion_percentage: number;
  eta_seconds?: number;
  latest_message: string;
  stages: StageProgress[];
  /** PDF extraction progress (only present for PDF documents) */
  pdf_progress?: PdfProgress;
  /** Chunk extraction progress (present during extraction stage) */
  chunk_progress?: ChunkProgress;
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
  type: "heartbeat" | "Heartbeat";
  timestamp: string;
  server_time: string;
}

/**
 * Connection confirmation event sent by backend when WebSocket connects.
 */
export interface ConnectedEvent {
  type: "Connected";
  timestamp: string;
  server_version?: string;
}

/**
 * Status snapshot event containing current pipeline state.
 *
 * WHY: Provides full synchronization of pipeline state when client connects
 * or reconnects, ensuring UI shows accurate status even after disconnection.
 */
export interface StatusSnapshotEvent {
  type: "StatusSnapshot";
  timestamp: string;
  active_tasks: Array<{
    track_id: string;
    document_id: string;
    status: string;
    progress: number;
  }>;
}

/**
 * PDF page-by-page progress event (OODA-PERF-02 optimization).
 *
 * WHY: PDF extraction can take 1-2 seconds per page. This event provides
 * page-level granularity so users see progress during the PDF→Markdown phase.
 * Debounced to send updates every 5 pages to reduce WebSocket traffic.
 */
export interface PdfPageProgressEvent {
  type: "PdfPageProgress";
  timestamp: string;
  data: {
    /** Document/PDF being processed */
    document_id: string;
    /** Task tracking ID (matches track_id for ingestion tracking) */
    task_id: string;
    /** Current page number */
    current_page: number;
    /** Total pages in PDF */
    total_pages: number;
    /** Progress percentage (0.0 - 1.0) */
    progress: number;
  };
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

/**
 * Chunk extraction failure event for resilient processing visibility.
 *
 * @implements SPEC-003: Chunk-level resilience with failure visibility
 *
 * WHY: When using process_with_resilience, some chunks may fail while
 * others succeed. This event notifies the frontend about individual
 * chunk failures, enabling:
 * - UI display of which chunks failed
 * - Error details for debugging
 * - Potential retry functionality
 */
export interface ChunkFailureEvent {
  type: "ChunkFailure";
  data: {
    /** Document being processed */
    document_id: string;
    /** Task tracking ID */
    task_id: string;
    /** Failed chunk index (0-based) */
    chunk_index: number;
    /** Total chunks in document */
    total_chunks: number;
    /** Error message describing the failure */
    error_message: string;
    /** Whether the failure was due to timeout */
    was_timeout: boolean;
    /** Number of retry attempts before giving up */
    retry_attempts: number;
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
  | ConnectedEvent
  | StatusSnapshotEvent
  | PdfPageProgressEvent
  | ChunkProgressEvent
  | ChunkFailureEvent;

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
