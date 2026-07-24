/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { getRuntimeServerBaseUrl } from "@/lib/runtime-config";
import { postMultipart, type MultipartUploadProgress } from "@/lib/upload/multipart-upload-client";
import { buildPdfUploadFormData } from "@/lib/upload/pdf-upload-form-data";
import { api, DOCUMENTS_API_TIMEOUT_MS } from "../client";
import { buildQueryString, withQuery } from "../query-params";

import type {
    Document,
    DocumentStatusCounts,
    ListDocumentsResponse,
    PaginatedResponse,
    PaginationParams,
    PdfUploadOptions,
    PdfUploadResponse,
    UploadDocumentRequest,
    UploadDocumentResponse,
} from "@/types";

/** Extended paginated response that includes status_counts from the server. */
export interface DocumentsListResult extends PaginatedResponse<Document> {
  status_counts: DocumentStatusCounts;
}

export async function getDocuments(
  params?: PaginationParams & {
    status?: string;
    /** @implements SPEC-005 */
    date_from?: string;
    /** @implements SPEC-005 */
    date_to?: string;
    /** @implements SPEC-005 */
    document_pattern?: string;
  },
): Promise<DocumentsListResult> {
  const query = buildQueryString({
    page: params?.page,
    page_size: params?.page_size,
    sort_by: params?.sort_by,
    sort_order: params?.sort_order,
    status: params?.status,
    date_from: params?.date_from,
    date_to: params?.date_to,
    document_pattern: params?.document_pattern,
  });

  // API now returns { documents: [...], total, page, page_size, total_pages, has_more, status_counts }
  // Fail-fast: never spin skeletons until the browser TCP timeout under ingest load.
  const response = await api.get<ListDocumentsResponse>(
    withQuery("/documents", query),
    { timeoutMs: DOCUMENTS_API_TIMEOUT_MS },
  );

  return {
    items: response.documents || [],
    total: response.total || 0,
    page: response.page || 1,
    page_size: response.page_size || 20,
    total_pages:
      response.total_pages ||
      Math.ceil((response.total || 0) / (response.page_size || 20)),
    has_more:
      response.has_more ?? response.page * response.page_size < response.total,
    status_counts: response.status_counts || {
      pending: 0,
      processing: 0,
      completed: 0,
      failed: 0,
    },
  };
}

export async function getDocument(documentId: string): Promise<Document> {
  return api.get<Document>(`/documents/${documentId}`, {
    timeoutMs: DOCUMENTS_API_TIMEOUT_MS,
  });
}

export async function uploadDocument(
  data: UploadDocumentRequest,
): Promise<UploadDocumentResponse> {
  return api.post<UploadDocumentResponse>("/documents", data);
}

export async function uploadFile(
  file: File,
  options?: { onUploadProgress?: (progress: MultipartUploadProgress) => void },
): Promise<UploadDocumentResponse> {
  const formData = new FormData();
  formData.append("file", file);

  return postMultipart<UploadDocumentResponse>("/documents/upload", formData, {
    fileSizeBytes: file.size,
    onProgress: options?.onUploadProgress,
  });
}

export type PdfUploadRequestOptions = PdfUploadOptions & {
  onUploadProgress?: (progress: MultipartUploadProgress) => void;
};

/**
 * Upload a PDF document for vision-based extraction.
 * @param file The PDF file to upload
 * @param options Upload options (vision settings, title, metadata)
 * @returns PDF upload response with processing status
 */
export async function uploadPdfDocument(
  file: File,
  options?: PdfUploadRequestOptions,
): Promise<PdfUploadResponse> {
  const formData = buildPdfUploadFormData(file, options);

  return postMultipart<PdfUploadResponse>("/documents/pdf", formData, {
    fileSizeBytes: file.size,
    onProgress: options?.onUploadProgress,
  });
}

/**
 * Response type for PDF progress endpoint.
 * Matches backend PdfUploadProgress struct (edgequake-tasks/src/progress.rs).
 *
 * WHY: The backend serializes is_complete/is_failed booleans (not a status
 * string), and phases as PhaseProgress objects (not a tagged union).
 * The hook computes a normalized `status` string from these fields.
 *
 * @implements OODA-19: PDF progress API integration
 */
export interface PdfProgressResponse {
  track_id: string;
  pdf_id: string;
  document_id?: string | null;
  filename: string;
  /** Computed by usePdfProgress hook from is_complete / is_failed */
  status?: "pending" | "processing" | "completed" | "failed";
  phases: PhaseProgressData[];
  overall_percentage: number;
  is_complete: boolean;
  is_failed: boolean;
  started_at: string;
  updated_at: string;
  completed_at?: string | null;
  eta_seconds?: number | null;
  /** Top-level error (set from the first failed phase message) */
  error?: string;
}

/**
 * Phase progress data from the backend PhaseProgress struct.
 *
 * WHY: Backend serializes phase status as `status: "active" | "complete" | ...`
 * (not a tagged union `type`), and uses `percentage` (not `percent`).
 * A `message` field carries real-time human-readable progress text.
 */
export interface PhaseProgressData {
  /** Phase identifier: "upload" | "pdf_conversion" | "chunking" | "embedding" | "extraction" | "graph_storage" */
  phase: string;
  /** Phase status: "pending" | "active" | "complete" | "failed" | "skipped" */
  status: "pending" | "active" | "complete" | "failed" | "skipped";
  current: number;
  total: number;
  /** Completion percentage 0–100 */
  percentage: number;
  /** Human-readable progress message, e.g. "Converting PDF: page 5/23 (22%)" */
  message: string;
  eta_seconds?: number | null;
  error?: PhaseErrorData | null;
  started_at?: string | null;
  completed_at?: string | null;
}

/**
 * Error details for a failed phase.
 */
export interface PhaseErrorData {
  message: string;
  code: string;
  retryable: boolean;
  suggestion: string;
  affected_item?: string | null;
}

/**
 * Legacy PhaseStatus discriminated union kept for backward compatibility
 * with components that haven't been updated yet.
 *
 * @deprecated Use PhaseProgressData instead.
 */
export type PhaseStatus =
  | { type: "pending" }
  | { type: "active"; current: number; total: number; percent: number }
  | { type: "completed" }
  | { type: "failed"; error: string };

/**
 * Response type for PDF retry/cancel operations.
 */
export interface PdfOperationResponse {
  success: boolean;
  pdf_id: string;
  message: string;
  task_id?: string;
}

/**
 * Get PDF upload progress by track ID.
 *
 * @implements OODA-19: PDF progress tracking
 * @param trackId The upload tracking ID
 * @returns Progress state with phase details
 */
export async function getPdfProgress(
  trackId: string,
): Promise<PdfProgressResponse> {
  return api.get<PdfProgressResponse>(`/documents/pdf/progress/${trackId}`);
}

/**
 * Create an EventSource for SSE-based progress streaming.
 * Preferred over polling for large documents (100+ pages).
 *
 * @implements FEAT-PDF-PROGRESS: SSE real-time page progress
 * @param trackId The upload tracking ID
 * @returns EventSource instance (caller is responsible for closing)
 */
export function createPdfProgressEventSource(trackId: string): EventSource {
  const baseUrl = getRuntimeServerBaseUrl();
  const url = `${baseUrl}/api/v1/documents/pdf/progress/stream/${trackId}`;
  return new EventSource(url);
}

/**
 * Retry a failed PDF processing.
 *
 * @implements OODA-19: PDF retry functionality
 * @param pdfId The PDF document ID
 * @returns Operation result with new task ID
 */
export async function retryPdfProcessing(
  pdfId: string,
): Promise<PdfOperationResponse> {
  return api.post<PdfOperationResponse>(`/documents/pdf/${pdfId}/retry`);
}

/**
 * Cancel an in-progress PDF processing.
 *
 * @implements OODA-19: PDF cancel functionality
 * @param pdfId The PDF document ID
 * @returns Operation result
 */
export async function cancelPdfProcessing(
  pdfId: string,
): Promise<PdfOperationResponse> {
  return api.delete<PdfOperationResponse>(`/documents/pdf/${pdfId}/cancel`);
}

/**
 * PDF content response with metadata and extracted markdown.
 *
 * @implements SPEC-002: Document Viewer
 */
export interface PdfContentResponse {
  /** PDF ID */
  pdf_id: string;
  /** Linked document ID (mm-asset scope for the markdown viewer) */
  document_id?: string | null;
  /** Original filename */
  filename: string;
  /** File size in bytes */
  file_size_bytes: number;
  /** MIME type (typically application/pdf) */
  content_type: string;
  /** Extracted markdown content (if processed) */
  markdown_content: string | null;
  /** Whether PDF processing is complete */
  is_processed: boolean;
}

/**
 * Get PDF content metadata including extracted markdown.
 *
 * @implements SPEC-002: Document Viewer - get PDF metadata with markdown
 * @param pdfId The PDF document ID
 * @returns PDF content response with metadata and markdown
 */
export async function getPdfContent(
  pdfId: string,
): Promise<PdfContentResponse> {
  return api.get<PdfContentResponse>(`/documents/pdf/${pdfId}/content`);
}

/**
 * Get the URL for downloading a PDF file.
 *
 * @implements SPEC-002: Document Viewer - PDF download URL
 * @param pdfId The PDF document ID
 * @returns Full URL to download the PDF
 */
export function getPdfDownloadUrl(pdfId: string): string {
  const baseUrl = getRuntimeServerBaseUrl();
  return `${baseUrl}/api/v1/documents/pdf/${pdfId}/download`;
}

/**
 * Get the URL for downloading a non-PDF original file.
 *
 * @implements SPEC-002 - Document original download
 */
export function getDocumentOriginalDownloadUrl(documentId: string): string {
  const baseUrl = getRuntimeServerBaseUrl();
  return `${baseUrl}/api/v1/documents/${documentId}/download/original`;
}

/**
 * Get the URL for downloading document markdown from the server.
 *
 * @implements SPEC-002 - Document markdown download
 */
export function getDocumentMarkdownDownloadUrl(documentId: string): string {
  const baseUrl = getRuntimeServerBaseUrl();
  return `${baseUrl}/api/v1/documents/${documentId}/download/markdown`;
}

/**
 * URL for a vision page / chart-crop PNG under the document.
 * Prefer stable asset-id REST (`…/assets/{asset_id}`); path form remains for splat.
 */
export function getDocumentMmAssetUrl(documentId: string, assetRelPath: string): string {
  const baseUrl = getRuntimeServerBaseUrl();
  const cleaned = assetRelPath.replace(/^\/+/, "");
  const stem = cleaned.split("/").pop()?.replace(/\.[^.]+$/, "") ?? cleaned;
  if (stem && !stem.includes("..")) {
    return `${baseUrl}/api/v1/documents/${documentId}/assets/${encodeURIComponent(stem)}`;
  }
  return `${baseUrl}/api/v1/documents/${documentId}/mm-assets/${cleaned}`;
}

/** List persisted mm-asset summaries for a document (no binary). */
export async function listDocumentAssets(
  documentId: string,
): Promise<{ document_id: string; assets: Array<{ asset_id?: string; path?: string }> }> {
  return api.get(`/documents/${documentId}/assets`);
}

/**
 * First principles: render page PNGs from the linked PDF, persist mm-assets,
 * and enrich markdown figure headings with `![…](assets/…)`.
 */
export async function includeDocumentAssetsFromPdf(
  documentId: string,
): Promise<{
  document_id: string;
  pages_rendered: number;
  assets_persisted: number;
  markdown_updated: boolean;
}> {
  return api.post(`/documents/${documentId}/assets/include-from-pdf`);
}

/** True when href already points at durable mm-assets (relative or API). */
export function isDurableMmAssetHref(href: string): boolean {
  const u = href.trim();
  return (
    u.startsWith("assets/") ||
    u.includes("/mm-assets/") ||
    /\/documents\/[^/]+\/assets\//.test(u)
  );
}

/**
 * Prefer existing figure/chart/table crops — never invent missing asset paths.
 * Inventing `page-NNNN-fig-01.png` when the file was never extracted causes broken images.
 */
function preferredViewerAsset(page: number, chunk: string): string | null {
  const durable = chunk.match(
    /!?\[[^\]]*\]\((assets\/page-\d{4}-(?:fig-\d{2}|chart|table-\d{2})\.png)\)/i,
  );
  if (durable?.[1]) return durable[1];
  void page;
  return null;
}

/**
 * Bind VLM-hallucinated / empty figure links to figure/chart crops using
 * `<!-- edgequake-page:N -->` markers (DRY with edgequake_pdf::bind_figure_images_to_page_asset).
 * Also injects images after `Figure N` headings that lack a nearby image.
 * Never invents full-page `page-NNNN.png` images (PDF pane already shows pages).
 */
export function bindFigureImagesToPageAssets(markdown: string): string {
  if (!markdown) return markdown;
  const pageRe = /<!--\s*edgequake-page:(\d+)\s*-->/g;
  const markers: { index: number; page: number }[] = [];
  let m: RegExpExecArray | null;
  while ((m = pageRe.exec(markdown)) !== null) {
    markers.push({ index: m.index, page: parseInt(m[1], 10) });
  }
  if (markers.length === 0) {
    const asset = preferredViewerAsset(1, markdown);
    if (!asset) return markdown;
    return injectFigureLocalImages(
      markdown.replace(
        /!\[([^\]]*)\]\(([^)]*)\)/g,
        (full, alt: string, href: string) => {
          if (isDurableMmAssetHref(href)) return full;
          return `![${alt}](${asset})`;
        },
      ),
      asset,
    );
  }

  let out = "";
  for (let i = 0; i < markers.length; i++) {
    const start = markers[i].index;
    const end = i + 1 < markers.length ? markers[i + 1].index : markdown.length;
    const page = markers[i].page;
    const chunk = markdown.slice(start, end);
    const asset = preferredViewerAsset(page, chunk);
    if (!asset) {
      out += chunk;
      continue;
    }
    const bound = chunk.replace(
      /!\[([^\]]*)\]\(([^)]*)\)/g,
      (full, alt: string, href: string) => {
        if (isDurableMmAssetHref(href)) return full;
        return `![${alt || "Page image"}](${asset})`;
      },
    );
    out += injectFigureLocalImages(bound, asset);
  }
  // Prefix before first page marker
  if (markers[0].index > 0) {
    out = markdown.slice(0, markers[0].index) + out;
  }
  return out;
}

function isFigureHeadingLine(line: string): boolean {
  const stripped = line
    .trim()
    .replace(/^#+\s*/, "")
    .replace(/^[*_]+/, "")
    .replace(/[*_]+$/, "")
    .trim();
  return /^figure\s+\d/i.test(stripped);
}

function figureHeadingAlt(line: string): string {
  const alt = line
    .trim()
    .replace(/^#+\s*/, "")
    .replace(/^\*+|\*+$/g, "")
    .trim()
    .slice(0, 120)
    .replaceAll("[", "")
    .replaceAll("]", "");
  return alt || "Figure";
}

/** Inject `![Figure…](assets/…)` after figure headings without a nearby image. */
export function injectFigureLocalImages(
  markdown: string,
  assetRelPath: string,
): string {
  if (!markdown.trim()) return markdown;
  const lines = markdown.split("\n");
  const out: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    out.push(line);
    if (!isFigureHeadingLine(line)) continue;
    let j = i + 1;
    while (j < lines.length && lines[j].trim() === "") j++;
    const next = j < lines.length ? lines[j].trimStart() : "";
    const hasNearby =
      next.startsWith("![") ||
      next.includes("](assets/") ||
      next.startsWith("<drawing");
    if (!hasNearby) {
      out.push("");
      out.push(`![${figureHeadingAlt(line)}](${assetRelPath})`);
    }
  }
  return out.join("\n");
}

/** Strip analyze-only `<drawing …/>` tags so they never leak into the viewer. */
export function stripDrawingTags(markdown: string): string {
  return markdown.replace(/<drawing\b[^>]*\/?\s*>/gi, "").replace(/\n{3,}/g, "\n\n");
}

/**
 * Rewrite `![alt](assets/…)` to authenticated API URLs for the markdown viewer.
 * Also binds hallucinated figure hrefs / injects figure-local images when page markers exist.
 */
export function rewriteMarkdownMmAssetUrls(
  markdown: string,
  documentId: string | null | undefined,
): string {
  if (!markdown || !documentId) return markdown;
  const bound = stripDrawingTags(bindFigureImagesToPageAssets(markdown));
  return bound.replace(
    /!\[([^\]]*)\]\((assets\/[^)\s]+)\)/g,
    (_m, alt: string, rel: string) => `![${alt}](${getDocumentMmAssetUrl(documentId, rel)})`,
  );
}

/** Async delete admit response (HTTP 202). Terminal state arrives via WebSocket. */
export interface DeleteDocumentAccepted {
  document_id: string;
  deleted: boolean;
  accepted: boolean;
  track_id?: string | null;
  chunks_deleted: number;
  entities_affected: number;
  relationships_affected: number;
  embeddings_deleted?: number;
  partial_failure?: boolean;
  partial_failure_reason?: string | null;
}

export async function deleteDocument(
  documentId: string,
): Promise<DeleteDocumentAccepted> {
  // Fast admit — cascade runs as a background job; do not wait on graph size.
  return api.delete<DeleteDocumentAccepted>(`/documents/${documentId}`);
}

export interface DeleteAllDocumentsResponse {
  /** True when wipe was accepted asynchronously (HTTP 202). */
  accepted?: boolean;
  /** Durable WorkspaceWipe task track id for WS correlation / poll. */
  wipe_track_id?: string;
  /** Planned delete count at admit time (final counts arrive via WS/task). */
  deleted_count: number;
  total_chunks_deleted?: number;
  total_entities_removed?: number;
  total_relationships_removed?: number;
  total_pdfs_deleted?: number;
  skipped_count?: number;
  skipped_documents?: string[];
}

export async function deleteAllDocuments(): Promise<DeleteAllDocumentsResponse> {
  // ISSUE-309: server returns 202 + wipe_track_id; terminal via WS or task poll.
  return api.delete<DeleteAllDocumentsResponse>("/documents");
}

/** SPEC-084 / GH-317: selected multi-delete (one durable batch task). */
export interface BatchDeleteDocumentsResponse {
  accepted: boolean;
  batch_track_id: string;
  planned_delete_count: number;
}

export async function batchDeleteDocuments(
  documentIds: string[],
): Promise<BatchDeleteDocumentsResponse> {
  return api.post<BatchDeleteDocumentsResponse>("/documents/batch-delete", {
    document_ids: documentIds,
  });
}

// ---------------------------------------------------------------------------
// Deletion Impact Analysis (SPEC-050)
// ---------------------------------------------------------------------------

/**
 * Pre-delete impact analysis — shows what will be removed without actually deleting.
 *
 * @implements SPEC-050: Show impact before confirm.
 * @implements SPEC-050/EC-2: Shared entity semantics.
 */
export interface DeletionImpact {
  /** Document ID being analysed. */
  document_id: string;
  /** Number of chunks (KV records) that would be deleted. */
  chunks_to_delete: number;
  /**
   * Entities that would be fully removed (no other source documents).
   * SPEC-050/EC-1: These entities exist ONLY in this document → DELETED.
   */
  entities_to_remove: number;
  /**
   * Entities that would be updated (still referenced by other documents).
   * SPEC-050/EC-2: These entities SURVIVE with pruned source_ids.
   * They are NOT deleted — they persist in the graph with fewer supporting sources.
   */
  entities_to_update: number;
  /**
   * Relationships that would be fully removed.
   * Includes exclusive relationships AND those whose endpoint entity is removed.
   */
  relationships_to_remove: number;
  /**
   * Relationships that would be updated (still referenced by other documents).
   * SPEC-050/EC-6: These relationships SURVIVE with pruned source_ids.
   */
  relationships_to_update: number;
  /** Always true — this endpoint is read-only. */
  preview_only: boolean;
}

/**
 * Fetch deletion impact for a document without deleting it.
 *
 * @implements SPEC-050: Pre-delete impact analysis.
 * @param documentId The document to analyse.
 */
export async function getDeletionImpact(
  documentId: string,
): Promise<DeletionImpact> {
  return api.get<DeletionImpact>(`/documents/${documentId}/deletion-impact`);
}

/**
 * Reprocess intent for a document.
 * - `entities`: reuse cached markdown, only re-run the KG pipeline (default).
 * - `full`: re-run PDF -> markdown conversion from the stored PDF bytes
 *   (spends vision tokens) before re-running the KG pipeline.
 */
export type ReprocessMode = "entities" | "full";

/**
 * Reprocess a single document by its document ID.
 * Uses the reprocess endpoint with document_id filter and force flag.
 * @param documentId The ID of the document to reprocess
 * @param force Whether to force reprocess even if document is not failed (default: true)
 * @param mode Reprocess intent: "entities" (reuse markdown) or "full" (re-convert PDF)
 */
export async function reprocessDocument(
  documentId: string,
  force: boolean = true,
  mode: ReprocessMode = "entities",
): Promise<ReprocessFailedResponse> {
  return api.post<ReprocessFailedResponse>("/documents/reprocess", {
    document_id: documentId,
    force,
    max_documents: 1,
    mode,
  });
}

/**
 * Scan input directory for new documents.
 * Triggers background scanning and processing of new files.
 * @param path Optional path to scan (defaults to configured input directory)
 */
export async function scanDocuments(
  path?: string,
): Promise<{ track_id: string; message: string }> {
  return api.post<{ track_id: string; message: string }>(
    "/documents/scan",
    path ? { path } : {},
  );
}

/**
 * Per-document progress identity from a reprocess response (SPEC-054 SSOT).
 */
export interface ReprocessDocumentTaskId {
  document_id: string;
  /** Server task track_id — sole progress / cancel / WS key */
  task_id: string;
}

/**
 * Response from reprocess failed documents endpoint.
 *
 * @implements OODA-37 - Fixed response type to match backend ReprocessFailedResponse
 */
export interface ReprocessFailedResponse {
  /** Batch correlation id (`reprocess_*`) — NOT a progress subscription key */
  track_id: string;
  /** Number of failed documents found */
  failed_found: number;
  /** Number of documents queued for reprocessing */
  requeued: number;
  /** Documents matched but not enqueued */
  skipped?: number;
  /** Counts by skip reason when requeued is less than expected */
  skip_reasons?: Record<string, number>;
  /** List of document IDs being reprocessed */
  document_ids: string[];
  /** Progress key when exactly one document was requeued (DRY with upload task_id) */
  task_id?: string | null;
  /** Per-document task ids for multi-doc honesty */
  document_task_ids?: ReprocessDocumentTaskId[];
}

/**
 * Reprocess all failed documents.
 * Retries processing of documents that previously failed.
 *
 * Sends an empty JSON body `{}` so Axum's Json<T> extractor does not
 * reject the request with 400 (empty body is not valid JSON).
 *
 * @returns ReprocessFailedResponse with track_id, counts, and document_ids
 */
export async function reprocessFailedDocuments(): Promise<ReprocessFailedResponse> {
  // WHY: Backend requires a JSON body (even empty {}); sending no body causes HTTP 400
  // "EOF while parsing a value at line 1 column 0"
  return api.post<ReprocessFailedResponse>("/documents/reprocess", {});
}

/**
 * Response from retry chunks endpoint.
 */
export interface RetryChunksResponse {
  /** Document ID */
  document_id: string;
  /** Number of chunks queued for retry */
  chunks_queued: number;
  /** Specific chunk indices being retried */
  chunk_indices: number[];
  /** Status message */
  message: string;
  /** Whether the feature is fully implemented */
  implemented: boolean;
}

/**
 * Information about a failed chunk.
 */
export interface FailedChunkApiInfo {
  /** Chunk index within the document */
  chunk_index: number;
  /** Chunk identifier */
  chunk_id: string;
  /** Error message from the failed extraction */
  error_message: string;
  /** Whether the failure was due to timeout */
  was_timeout: boolean;
  /** Number of retry attempts so far */
  retry_attempts: number;
  /** Current status: pending, retrying, succeeded, abandoned */
  status: string;
}

/**
 * Response from list failed chunks endpoint.
 */
export interface ListFailedChunksResponse {
  /** Document ID */
  document_id: string;
  /** List of failed chunks */
  failed_chunks: FailedChunkApiInfo[];
  /** Total number of chunks in the document */
  total_chunks: number;
  /** Number of successful chunks */
  successful_chunks: number;
}

/**
 * Retry failed chunks for a specific document.
 *
 * @implements OODA-03 - Chunk-level retry queue
 *
 * Note: This is a scaffolding endpoint. Full implementation pending.
 * Currently returns a placeholder response with implemented=false.
 *
 * @param documentId The ID of the document
 * @param chunkIndices Specific chunk indices to retry. If empty, retries all failed chunks.
 * @param force Whether to force retry even if chunk already succeeded
 */
export async function retryFailedChunks(
  documentId: string,
  chunkIndices: number[] = [],
  force: boolean = false,
): Promise<RetryChunksResponse> {
  return api.post<RetryChunksResponse>(
    `/documents/${documentId}/retry-chunks`,
    { chunk_indices: chunkIndices, force, max_retries: 3 },
  );
}

/**
 * List failed chunks for a document.
 *
 * @implements OODA-03 - Chunk-level retry queue
 *
 * @param documentId The ID of the document
 */
export async function listFailedChunks(
  documentId: string,
): Promise<ListFailedChunksResponse> {
  return api.get<ListFailedChunksResponse>(
    `/documents/${documentId}/failed-chunks`,
  );
}

/**
 * Search documents by title for the scope picker (type-ahead).
 * Returns minimal projections (id, title, status) — no chunk/entity counts.
 * @implements SPEC-031: Document search endpoint
 */
export async function searchDocuments(params: {
  q?: string;
  page_size?: number;
  status?: string;
}): Promise<import("@/types").DocumentSearchResponse> {
  const query = buildQueryString({
    q: params.q,
    page_size: params.page_size ?? 20,
    status: params.status ?? "completed",
  });
  return api.get<import("@/types").DocumentSearchResponse>(
    withQuery("/documents/search", query),
  );
}
