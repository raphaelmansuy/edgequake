/** Document ingestion, upload, and pipeline status types. */

import type { PdfParserBackend } from "./graph";

export interface Document {
  id: string;
  title?: string | null;
  content?: string;
  source_type?: "file" | "text" | "url" | "pdf" | "markdown" | "image";
  status?:
    | "pending"
    | "processing"
    | "completed"
    | "partial_failure"
    | "failed"
    | "indexed"
    | "cancelled";
  error_message?: string;
  /** Structured failure code (e.g. `server_restart_interrupted`). */
  failure_code?: string;
  /** Non-fatal processing notice (e.g. vision parser fallback). */
  warning_message?: string;
  file_name?: string;
  file_size?: number;
  mime_type?: string;
  chunk_count?: number;
  entity_count?: number;
  /** Number of relationships extracted. */
  relationship_count?: number;
  /** First 200 characters of document content (preview). */
  content_summary?: string;
  /** Total length of document content in characters. */
  content_length?: number;
  /** Content hash for deduplication (SHA-256). */
  content_hash?: string;
  /** Track ID for batch grouping. */
  track_id?: string;
  /**
   * True while document lives in `staging:{id}-metadata` (pre-promote).
   * SPEC-086: list-visible; orphan recovery fails shells with no live task.
   */
  admission_staging?: boolean;
  /** Tenant ID for multi-tenancy. */
  tenant_id?: string;
  /** Workspace ID for multi-tenancy. */
  workspace_id?: string;
  created_at?: string;
  updated_at?: string;
  processed_at?: string;
  /** Extraction lineage information. */
  lineage?: DocumentLineage;
  /** Total processing cost in USD. */
  cost_usd?: number;
  /** Input tokens used for processing. */
  input_tokens?: number;
  /** Output tokens used for processing. */
  output_tokens?: number;
  /** Total tokens (input + output). */
  total_tokens?: number;
  /** LLM model used for processing. */
  llm_model?: string;
  /** Embedding model used for processing. */
  embedding_model?: string;

  // ========================================================================
  // OODA-10: Enhanced Lineage Metadata Fields
  // ========================================================================

  /** Document type (pdf, markdown, text). @implements F1 */
  document_type?: string;
  /** SHA-256 checksum for integrity verification. @implements F2 */
  sha256_checksum?: string;
  /** Number of pages (PDF documents only). @implements F2 */
  page_count?: number;
  /** File size in bytes (from metadata). @implements F1 */
  file_size_bytes?: number;

  // ========================================================================
  // SPEC-002: Unified Ingestion Pipeline Fields
  // ========================================================================

  /**
   * Current ingestion stage (aligned with UnifiedStage enum).
   * Stages: uploading, converting, preprocessing, chunking, extracting,
   * gleaning, merging, summarizing, embedding, storing, completed, failed.
   * @implements SPEC-002
   */
  current_stage?: string;

  /**
   * Progress within current stage (0.0 to 1.0).
   * @implements SPEC-002
   */
  stage_progress?: number;

  /**
   * Human-readable message for current stage.
   * @implements SPEC-002
   */
  stage_message?: string;

  /**
   * SPEC-057 P4: API SSOT badge key (cancelled|failed|completed|extracting|…).
   * Prefer over local stage/status derivation when present.
   */
  display_status?: string;

  /**
   * SPEC-057 P4: idle|running|stopping|terminal — Stopping… when `stopping`.
   */
  ui_phase?: string;

  /**
   * Reprocess mode when this run was started via soft/hard reprocess (SPEC-048).
   * Wire: full | entities | merge
   */
  reprocess_mode?: string;

  /**
   * Linked PDF document ID (only set if source_type is "pdf").
   * Used to fetch PDF content for viewing.
   * @implements SPEC-002
   */
  pdf_id?: string;

  /**
   * Opaque server metadata bag (e.g. `has_original` for download eligibility).
   * @implements SPEC-002
   */
  metadata?: Record<string, unknown>;
}

/** Extraction lineage information for a document. */
export interface DocumentLineage {
  /** LLM model used for entity extraction. */
  llm_model?: string;
  /** Embedding model used for vector embeddings. */
  embedding_model?: string;
  /** Embedding dimensions. */
  embedding_dimensions?: number;
  /** List of keywords extracted. */
  keywords?: string[];
  /** Entity types extracted. */
  entity_types?: string[];
  /** Relationship types extracted. */
  relationship_types?: string[];
  /** Chunking strategy used. */
  chunking_strategy?: string;
  /** Average chunk size in characters. */
  avg_chunk_size?: number;
  /** Processing duration in milliseconds. */
  processing_duration_ms?: number;
  /** Entity extraction duration in milliseconds. */
  entity_extraction_ms?: number;
  /** Relationship extraction duration in milliseconds. */
  relationship_extraction_ms?: number;
  /** Graph indexing duration in milliseconds. */
  graph_indexing_ms?: number;
  /** Vector embedding duration in milliseconds. */
  vector_embedding_ms?: number;
  /**
   * Vision LLM model used for PDF→Markdown extraction (PDF documents only).
   * Populated from pdf_vision_model metadata field set by the PDF processor.
   * @implements SPEC-040 - Workspace-level Vision LLM config
   */
  pdf_vision_model?: string;
  /**
   * PDF extraction method used: "vision" | "text" | "hybrid" (PDF documents only).
   * @implements SPEC-040
   */
  pdf_extraction_method?: string;
  /** PDF extraction warning surfaced from the backend. */
  pdf_extraction_warning?: string;
  /** Total tokens consumed. */
  total_tokens?: number;
  /** Input tokens consumed. */
  input_tokens?: number;
  /** Output tokens generated. */
  output_tokens?: number;
  /** Estimated cost in USD. */
  cost_usd?: number;
}

/** Status counts for document filtering. */
export interface DocumentStatusCounts {
  pending: number;
  processing: number;
  completed: number;
  partial_failure: number;
  failed: number;
  cancelled: number;
}

/** Response from list documents API. */
export interface ListDocumentsResponse {
  documents: Document[];
  total: number;
  page: number;
  page_size: number;
  /** Total number of pages. */
  total_pages?: number;
  /** Whether there are more pages after this one. */
  has_more?: boolean;
  status_counts: DocumentStatusCounts;
}

/** Track status response for batch grouping (Phase 2). */
export interface TrackStatusResponse {
  /** Track ID for this batch. */
  track_id: string;
  /** When the first document was uploaded. */
  created_at?: string;
  /** Documents in this batch. */
  documents: Document[];
  /** Total number of documents. */
  total_count: number;
  /** Status summary for the batch. */
  status_summary: DocumentStatusCounts;
  /** Whether processing is complete (all docs completed or failed). */
  is_complete: boolean;
  /** Latest processing message. */
  latest_message?: string;
}

/** Pipeline message from the server (Phase 3). */
export interface PipelineMessage {
  timestamp: string;
  level: "info" | "warn" | "error";
  message: string;
}

/** Enhanced pipeline status response (Phase 3). */
export interface EnhancedPipelineStatus {
  /** Whether the pipeline is currently processing. */
  is_busy: boolean;
  /** Current job name. */
  job_name?: string;
  /** When the current job started. */
  job_start?: string;
  /** Total documents to process. */
  total_documents: number;
  /** Documents processed so far. */
  processed_documents: number;
  /** Current batch number. */
  current_batch: number;
  /** Total number of batches. */
  total_batches: number;
  /** Latest status message. */
  latest_message?: string;
  /** History of pipeline messages. */
  history_messages: PipelineMessage[];
  /** Whether cancellation has been requested. */
  cancellation_requested: boolean;
  /** Number of pending tasks. */
  pending_tasks: number;
  /** Number of processing tasks. */
  processing_tasks: number;
  /** Number of completed tasks. */
  completed_tasks: number;
  /** Number of failed tasks. */
  failed_tasks: number;
}

/**
 * Queue metrics for Objective B: Workspace-Level Task Queue Visibility.
 *
 * @implements FEAT0570 - Queue metrics API
 * @implements OODA-21 - Queue metrics frontend integration
 */
export interface QueueMetrics {
  /** Number of pending tasks in the queue. */
  pending_count: number;
  /** Number of tasks currently being processed. */
  processing_count: number;
  /** Number of workers currently active. */
  active_workers: number;
  /** Maximum configured workers. */
  max_workers: number;
  /** Worker utilization percentage (0-100). */
  worker_utilization: number;
  /** Average wait time in seconds for recently started tasks. */
  avg_wait_time_seconds: number;
  /** Maximum wait time in seconds among pending tasks. */
  max_wait_time_seconds: number;
  /** Current throughput in documents per minute. */
  throughput_per_minute: number;
  /** Estimated time to clear the queue in seconds. */
  estimated_queue_time_seconds: number;
  /** Whether the system is currently rate limited. */
  rate_limited: boolean;
  /** When these metrics were captured (ISO 8601). */
  timestamp: string;
}

export interface DocumentChunk {
  id: string;
  document_id: string;
  content: string;
  chunk_index: number;
  tokens: number;
  embedding_id?: string;
}

export interface UploadDocumentRequest {
  content: string;
  title?: string;
  /** SPEC-086: include markdown (and file kinds) for text-path admission. */
  source_type?: "text" | "file" | "url" | "markdown" | "image" | "pdf";
  metadata?: Record<string, unknown>;
  async_processing?: boolean;
  /** Optional track ID for batch grouping. If not provided, one will be generated. */
  track_id?: string;
}

export interface UploadDocumentResponse {
  document_id: string;
  status: string;
  task_id?: string;
  /** Track ID for batch grouping. */
  track_id?: string;
  /** ID of existing document if this is a duplicate. */
  duplicate_of?: string;
  /** Multipart file upload: true when hash matches in-flight document. */
  is_duplicate?: boolean;
  chunk_count?: number;
  entity_count?: number;
  relationship_count?: number;
}

// PDF Upload types
export interface PdfUploadOptions {
  /** Enable vision LLM processing (default: true) */
  enable_vision?: boolean;
  /** Vision provider to use (default: "openai") */
  vision_provider?: string;
  /** Vision model override (optional) */
  vision_model?: string;
  /** Document title (optional) */
  title?: string;
  /** Custom metadata (optional) */
  metadata?: Record<string, unknown>;
  /** Batch tracking ID (optional) - OODA-19 */
  track_id?: string;
  /**
   * Force re-indexing of duplicate PDF (default: false).
   * WHY (OODA-08): When true, existing graph/vector data is cleared
   * and the document is re-processed with current LLM/config.
   * Used by duplicate Replace flow instead of DELETE + re-upload.
   * @implements BR-dup-replace - Replace = force_reindex on existing PDF
   */
  force_reindex?: boolean;
  /** Per-upload PDF parser backend override. Omit to use workspace/server default. */
  pdf_parser_backend?: PdfParserBackend;
  /**
   * LightRAG `process_options` string (e.g. `"i"` for inline image VLM analysis).
   * When omitted, `analyze_inline_images` controls whether `"i"` is sent.
   */
  process_options?: string;
  /** When true, sends `process_options=i` on PDF upload (inline image VLM). */
  analyze_inline_images?: boolean;
}

export interface PdfMetadata {
  filename: string;
  file_size_bytes: number;
  page_count?: number;
  sha256_checksum: string;
}

export interface PdfUploadResponse {
  pdf_id: string;
  document_id?: string;
  status: string;
  task_id: string;
  track_id?: string;
  message: string;
  estimated_time_seconds: number;
  metadata: PdfMetadata;
  duplicate_of?: string;
}

// ── SPEC-031: Lightweight document search types ───────────────────────────────

/**
 * Minimal document projection for the scope picker.
 * Returned by GET /api/v1/documents/search.
 * @implements SPEC-031
 */
export interface DocumentSearchItem {
  id: string;
  title: string;
  status: string;
  created_at?: string;
}

/** Response from GET /api/v1/documents/search. @implements SPEC-031 */
export interface DocumentSearchResponse {
  items: DocumentSearchItem[];
  total: number;
  has_more: boolean;
}
