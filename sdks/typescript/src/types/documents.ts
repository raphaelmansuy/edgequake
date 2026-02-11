/**
 * Document types.
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
  async_processing?: boolean;
  track_id?: string;
  enable_gleaning?: boolean;
  max_gleaning?: number;
  use_llm_summarization?: boolean;
}

export interface UploadDocumentResponse {
  document_id: string;
  status: string;
  track_id?: string;
  message?: string;
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
  limit?: number;
  offset?: number;
  status?: string;
  search?: string;
}

export interface DocumentInfo {
  document_id: string;
  title?: string;
  status: string;
  created_at: Timestamp;
  updated_at?: Timestamp;
  chunk_count?: number;
  entity_count?: number;
  content_length?: number;
}

export interface DocumentDetail extends DocumentInfo {
  content?: string;
  metadata?: Record<string, unknown>;
  track_id?: string;
  error_message?: string;
}

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
