/**
 * Document types.
 *
 * WHY: Updated to match Rust documents_types.rs exactly.
 * Rust types include detailed pipeline fields (source_type, current_stage,
 * stage_progress), cost info, and structured pagination.
 *
 * @module types/documents
 * @see edgequake/crates/edgequake-api/src/handlers/documents_types.rs
 */

import type { Timestamp } from "./common.js";

// ── Upload ────────────────────────────────────────────────────

export interface UploadDocumentRequest {
  content: string;
  title?: string;
  metadata?: Record<string, unknown>;
  /** Whether to process asynchronously (default: false). */
  async_processing?: boolean;
  /** Track ID for batch grouping. If not provided, one will be generated. */
  track_id?: string;
  /** Enable gleaning (multiple extraction passes). Default: true. */
  enable_gleaning?: boolean;
  /** Maximum gleaning passes (1-3 recommended). Default: 1. */
  max_gleaning?: number;
  /** Enable LLM-powered description summarization. Default: true. */
  use_llm_summarization?: boolean;
}

export interface UploadDocumentResponse {
  document_id: string;
  status: string;
  /** Task ID (only set when async_processing is true). */
  task_id?: string;
  /** Track ID for batch grouping. */
  track_id: string;
  /** ID of existing document if this is a duplicate. */
  duplicate_of?: string;
  /** Number of chunks created (only set for sync processing). */
  chunk_count?: number;
  /** Number of entities extracted (only set for sync processing). */
  entity_count?: number;
  /** Number of relationships extracted (only set for sync processing). */
  relationship_count?: number;
  /** Cost information (only set for sync processing). */
  cost?: DocumentCostInfo;
}

/** Cost information for a processed document. */
export interface DocumentCostInfo {
  /** Total cost in USD. */
  total_cost_usd: number;
  /** Formatted cost string (e.g., "$0.0045"). */
  formatted_cost: string;
  /** Total input tokens used. */
  input_tokens: number;
  /** Total output tokens used. */
  output_tokens: number;
  /** Total tokens (input + output). */
  total_tokens: number;
  /** LLM model used. */
  llm_model?: string;
  /** Embedding model used. */
  embedding_model?: string;
}

// ── File Upload ───────────────────────────────────────────────

export interface UploadFileMetadata {
  title?: string;
  metadata?: Record<string, unknown>;
}

export interface BatchUploadResponse {
  documents: UploadDocumentResponse[];
  total: number;
  succeeded: number;
  failed: number;
}

/** Response from single file upload. */
export type UploadFileResponse = UploadDocumentResponse;

// ── List ──────────────────────────────────────────────────────

export interface ListDocumentsQuery {
  /** Page number (1-indexed). Default: 1. */
  page?: number;
  /** Page size. Default: 20. */
  page_size?: number;
  /** Filter by status. */
  status?: string;
  /** Search text in title/content. */
  search?: string;
}

/** Status counts for document filtering. */
export interface StatusCounts {
  pending: number;
  processing: number;
  completed: number;
  /** Documents with partial failure (processed but 0 entities). */
  partial_failure: number;
  failed: number;
  cancelled: number;
}

/** List documents response. */
export interface ListDocumentsResponse {
  documents: DocumentSummary[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
  has_more: boolean;
  /** Status counts for all documents (not just current page). */
  status_counts: StatusCounts;
}

/** Document summary (list item). */
export interface DocumentSummary {
  id: string;
  title?: string;
  /** Original file name. */
  file_name?: string;
  /** First 200 characters of content. */
  content_summary?: string;
  /** Total content length in characters. */
  content_length?: number;
  /** Number of chunks. */
  chunk_count: number;
  /** Number of entities extracted. */
  entity_count?: number;
  /** Processing status (legacy — use current_stage for pipelines). */
  status?: string;
  /** Error message if processing failed. */
  error_message?: string;
  /** Track ID for batch grouping. */
  track_id?: string;
  created_at?: string;
  updated_at?: string;
  /** Total cost in USD. */
  cost_usd?: number;
  /** Input tokens used. */
  input_tokens?: number;
  /** Output tokens used. */
  output_tokens?: number;
  /** Total tokens (input + output). */
  total_tokens?: number;
  /** LLM model used. */
  llm_model?: string;
  /** Embedding model used. */
  embedding_model?: string;
  /** Document source type (pdf, markdown, text). */
  source_type?: string;
  /** Current ingestion stage - see docs for stage names. */
  current_stage?: string;
  /** Progress within current stage (0.0 to 1.0). */
  stage_progress?: number;
  /** Human-readable message for current stage. */
  stage_message?: string;
  /** Linked PDF document ID (only if source_type is "pdf"). */
  pdf_id?: string;
}

/** Document detail response with full content. */
export interface DocumentDetail {
  id: string;
  title?: string;
  file_name?: string;
  /** Full document content. */
  content?: string;
  content_summary?: string;
  content_length?: number;
  chunk_count: number;
  entity_count?: number;
  status?: string;
  error_message?: string;
  track_id?: string;
  created_at?: string;
  updated_at?: string;
  cost_usd?: number;
  input_tokens?: number;
  output_tokens?: number;
  total_tokens?: number;
  llm_model?: string;
  embedding_model?: string;
  source_type?: string;
  current_stage?: string;
  stage_progress?: number;
  stage_message?: string;
  pdf_id?: string;
  metadata?: Record<string, unknown>;
}

// ── Legacy aliases ────────────────────────────────────────────
/** @deprecated Use DocumentSummary */
export type DocumentInfo = DocumentSummary;

// ── Track ─────────────────────────────────────────────────────

export interface TrackStatusResponse {
  track_id: string;
  status: string;
  progress?: number;
  documents?: Array<{
    document_id: string;
    status: string;
    error?: string;
  }>;
  created_at?: Timestamp;
  updated_at?: Timestamp;
}

// ── Scan ──────────────────────────────────────────────────────

export interface ScanDirectoryRequest {
  path: string;
  recursive?: boolean;
  max_files?: number;
  extensions?: string[];
}

export interface ScanDirectoryResponse {
  total_files: number;
  queued: number;
  skipped: number;
  track_id: string;
}

// ── Reprocess ─────────────────────────────────────────────────

export interface ReprocessRequest {
  max_reprocess?: number;
}

export interface ReprocessResponse {
  reprocessed: number;
  message: string;
}

export interface RecoverStuckRequest {
  stuck_threshold_minutes?: number;
}

export interface RecoverStuckResponse {
  recovered: number;
  message: string;
}

// ── Deletion Impact ───────────────────────────────────────────

export interface DeletionImpactResponse {
  document_id: string;
  chunks_affected: number;
  entities_affected: number;
  relationships_affected: number;
}

// ── Chunks ────────────────────────────────────────────────────

export interface RetryChunksResponse {
  retried: number;
  message: string;
}

export interface FailedChunkInfo {
  chunk_id: string;
  error: string;
  created_at: Timestamp;
}

export interface FailedChunksResponse {
  chunks: FailedChunkInfo[];
  total: number;
}

// ── PDF ───────────────────────────────────────────────────────

/**
 * Options for PDF upload (v0.4.0+).
 *
 * Vision pipeline: renders each page to an image and sends it to a
 * multimodal LLM for high-fidelity Markdown extraction.
 */
export interface PdfUploadOptions {
  title?: string;
  metadata?: Record<string, unknown>;
  track_id?: string;
  /** Enable LLM vision pipeline for high-fidelity extraction. Default: false. */
  enable_vision?: boolean;
  /** Override vision provider (e.g. "openai", "ollama"). */
  vision_provider?: string;
  /** Override vision model (e.g. "gpt-4o", "gemma3"). */
  vision_model?: string;
  /** Re-process even if document already exists. Default: false. */
  force_reindex?: boolean;
}

/** @deprecated Use PdfUploadOptions — kept for backward compatibility. */
export interface PdfUploadMetadata {
  title?: string;
  metadata?: Record<string, unknown>;
}

export interface PdfUploadResponse {
  pdf_id: string;
  document_id?: string;
  status: string;
  track_id: string;
  message?: string;
}

export interface ListPdfsQuery {
  limit?: number;
  offset?: number;
  status?: string;
}

export interface PdfInfo {
  pdf_id: string;
  document_id?: string;
  filename: string;
  status: string;
  file_size: number;
  page_count?: number;
  created_at: Timestamp;
  /** Extraction method used: "vision", "text", or "ocr" (0.4.0+). */
  extraction_method?: string;
}

export interface PdfStatusResponse extends PdfInfo {
  error_message?: string;
  markdown_content?: string;
  track_id?: string;
}

export interface PdfContentResponse {
  pdf_id: string;
  markdown: string;
  page_count: number;
}

export interface PdfProgressResponse {
  track_id: string;
  status: string;
  progress: number;
  current_page?: number;
  total_pages?: number;
  message?: string;
}

export interface PdfRetryResponse {
  pdf_id: string;
  status: string;
  message: string;
}

// ── Delete All ────────────────────────────────────────────────

export interface DeleteAllResponse {
  deleted: number;
  message: string;
}
