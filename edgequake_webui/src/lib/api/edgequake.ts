/**
 * @module edgequake-api
 * @description TypeScript API client for EdgeQuake backend.
 * Provides typed functions for all REST endpoints with streaming support.
 *
 * @implements FEAT0007 - Query API with streaming responses
 * @implements FEAT0001 - Document upload and ingestion API
 * @implements FEAT0601 - Graph data API with SSE streaming
 * @implements FEAT0870 - Authentication API (login/logout)
 *
 * @enforces BR0001 - All API calls include tenant/workspace context
 * @enforces BR0002 - Error responses follow consistent format
 *
 * @see {@link specs/API.md} for endpoint specifications
 */
import type {
  CreateWorkspaceRequest,
  Document,
  DocumentStatusCounts,
  EnhancedPipelineStatus,
  Entity,
  GraphEdge,
  GraphNode,
  HealthResponse,
  KnowledgeGraph,
  ListDocumentsResponse,
  LoginRequest,
  LoginResponse,
  MergeEntitiesRequest,
  MergeEntitiesResponse,
  PaginatedResponse,
  PaginationParams,
  PdfUploadOptions,
  PdfUploadResponse,
  PipelineStatus,
  QueryRequest,
  QueryResponse,
  QueryStreamChunk,
  QueueMetrics,
  Relationship,
  Tenant,
  TrackStatusResponse,
  UploadDocumentRequest,
  UploadDocumentResponse,
  Workspace,
} from "@/types";
import { api, SERVER_BASE_URL, streamClient } from "./client";

// ============================================================================
// Health (These are at server root, not under /api/v1)
// ============================================================================

export async function checkHealth(): Promise<HealthResponse> {
  const url = SERVER_BASE_URL ? `${SERVER_BASE_URL}/health` : "/health";
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Health check failed: ${response.statusText}`);
  }
  return response.json();
}

export async function checkReady(): Promise<{ status: string }> {
  const url = SERVER_BASE_URL ? `${SERVER_BASE_URL}/ready` : "/ready";
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Readiness check failed: ${response.statusText}`);
  }
  return response.json();
}

// ============================================================================
// Authentication
// ============================================================================

export async function login(credentials: LoginRequest): Promise<LoginResponse> {
  return api.post<LoginResponse>("/auth/login", credentials);
}

export async function logout(): Promise<void> {
  return api.post<void>("/auth/logout");
}

export async function refreshToken(
  refreshToken: string,
): Promise<{ access_token: string; refresh_token: string }> {
  return api.post<{ access_token: string; refresh_token: string }>(
    "/auth/refresh",
    { refresh_token: refreshToken },
  );
}

export async function getCurrentUser(): Promise<LoginResponse["user"]> {
  return api.get<LoginResponse["user"]>("/auth/me");
}

// ============================================================================
// Tenants & Workspaces
// ============================================================================

/** Paginated tenant list response from backend. */
interface TenantListResponse {
  items: Tenant[];
  total: number;
  offset: number;
  limit: number;
}

/** Paginated workspace list response from backend. */
interface WorkspaceListResponse {
  items: Workspace[];
  total: number;
  offset: number;
  limit: number;
}

/** Workspace statistics response from backend. */
export interface WorkspaceStats {
  workspace_id: string;
  document_count: number;
  entity_count: number;
  relationship_count: number;
  /** Number of distinct entity types (e.g., PERSON, ORGANIZATION). */
  entity_type_count: number;
  chunk_count: number;
  embedding_count: number;
  storage_bytes: number;
}

export async function getTenants(): Promise<Tenant[]> {
  const response = await api.get<TenantListResponse | Tenant[]>("/tenants");
  // Handle both paginated response and legacy array format
  if (Array.isArray(response)) {
    return response;
  }
  return response.items || [];
}

export async function getTenant(tenantId: string): Promise<Tenant> {
  return api.get<Tenant>(`/tenants/${tenantId}`);
}

/**
 * Request to create a new tenant with optional model configuration.
 *
 * @implements SPEC-032: Tenant-level LLM and embedding model defaults
 */
export interface CreateTenantRequest {
  /** Tenant display name (required). */
  name: string;
  /** Optional description. */
  description?: string;
  /** Subscription plan (free, basic, pro, enterprise). */
  plan?: string;

  // === Default LLM Configuration (SPEC-032) ===

  /** Default LLM model for new workspaces (e.g., "gemma3:12b", "gpt-4o-mini"). */
  default_llm_model?: string;
  /** Default LLM provider for new workspaces ("ollama", "openai", "lmstudio"). */
  default_llm_provider?: string;

  // === Default Embedding Configuration (SPEC-032) ===

  /** Default embedding model for new workspaces (e.g., "text-embedding-3-small"). */
  default_embedding_model?: string;
  /** Default embedding provider for new workspaces ("openai", "ollama", "lmstudio"). */
  default_embedding_provider?: string;
  /** Default embedding dimension for new workspaces (e.g., 1536, 768). */
  default_embedding_dimension?: number;

  // === Default Vision LLM Configuration (SPEC-041) ===

  /** Default vision LLM model for new workspaces (e.g., "gpt-4o", "gemma3:12b"). */
  default_vision_llm_model?: string;
  /** Default vision LLM provider for new workspaces ("openai", "ollama"). */
  default_vision_llm_provider?: string;
}

/**
 * Create a new tenant with optional default model configuration.
 *
 * @implements SPEC-032: Tenant-level LLM and embedding model defaults
 *
 * @param data - Tenant creation request with optional model config
 * @returns Created tenant
 */
export async function createTenant(data: CreateTenantRequest): Promise<Tenant> {
  return api.post<Tenant>("/tenants", data);
}

export async function getWorkspaces(tenantId: string): Promise<Workspace[]> {
  const response = await api.get<WorkspaceListResponse | Workspace[]>(
    `/tenants/${tenantId}/workspaces`,
  );
  // Handle both paginated response and legacy array format
  if (Array.isArray(response)) {
    return response;
  }
  return response.items || [];
}

/**
 * Get a workspace by its ID.
 *
 * Note: The backend uses `/workspaces/{workspace_id}` without tenant prefix
 * for individual workspace operations. The tenant prefix is only used for
 * listing and creating workspaces.
 */
export async function getWorkspace(
  _tenantId: string,
  workspaceId: string,
): Promise<Workspace> {
  // Backend route: GET /api/v1/workspaces/{workspace_id}
  return api.get<Workspace>(`/workspaces/${workspaceId}`);
}

/**
 * Get a workspace by its URL-friendly slug.
 * Useful for URL-based workspace routing.
 */
export async function getWorkspaceBySlug(
  tenantId: string,
  slug: string,
): Promise<Workspace> {
  return api.get<Workspace>(`/tenants/${tenantId}/workspaces/by-slug/${slug}`);
}

export async function getWorkspaceStats(
  workspaceId: string,
): Promise<WorkspaceStats> {
  return api.get<WorkspaceStats>(`/workspaces/${workspaceId}/stats`);
}

/**
 * Create a new workspace with optional embedding configuration.
 *
 * @implements SPEC-032: Workspace-level embedding model selection
 *
 * @param tenantId - Parent tenant ID
 * @param data - Workspace creation request with optional embedding config
 * @returns Created workspace
 */
export async function createWorkspace(
  tenantId: string,
  data: CreateWorkspaceRequest,
): Promise<Workspace> {
  return api.post<Workspace>(`/tenants/${tenantId}/workspaces`, data);
}

/**
 * Request to update a workspace.
 * @implements SPEC-032: Workspace configuration update
 */
export interface UpdateWorkspaceRequest {
  /** New workspace name (optional) */
  name?: string;
  /** New description (optional) */
  description?: string;
  /** New LLM model (optional) */
  llm_model?: string;
  /** New LLM provider (optional) */
  llm_provider?: string;
  /** New embedding model (optional) */
  embedding_model?: string;
  /** New embedding provider (optional) */
  embedding_provider?: string;
  /** New embedding dimension (optional) */
  embedding_dimension?: number;
  /** Whether workspace is active (optional) */
  is_active?: boolean;
  /**
   * Vision LLM provider for PDF-to-Markdown extraction (e.g., "openai", "ollama").
   * @implements SPEC-040: Workspace-scoped Vision LLM for PDF processing
   */
  vision_llm_provider?: string;
  /**
   * Vision LLM model for PDF-to-Markdown extraction (e.g., "gpt-4o", "gemma3:12b").
   * @implements SPEC-040: Workspace-scoped Vision LLM for PDF processing
   */
  vision_llm_model?: string;
}

/**
 * Update an existing workspace.
 *
 * @implements SPEC-032: Workspace-level configuration update
 *
 * Note: Backend uses PUT /workspaces/{workspace_id} (no tenant prefix)
 *
 * @param _tenantId - Parent tenant ID (unused, kept for API compatibility)
 * @param workspaceId - Workspace ID to update
 * @param data - Update request
 * @returns Updated workspace
 */
export async function updateWorkspace(
  _tenantId: string,
  workspaceId: string,
  data: UpdateWorkspaceRequest,
): Promise<Workspace> {
  // Backend route: PUT /api/v1/workspaces/{workspace_id}
  return api.put<Workspace>(`/workspaces/${workspaceId}`, data);
}

// ============================================================================
// Rebuild Embeddings (SPEC-032)
// ============================================================================

/**
 * Request to rebuild workspace embeddings.
 */
export interface RebuildEmbeddingsRequest {
  /** New embedding model (optional, keeps current if not provided) */
  embedding_model?: string;
  /** New embedding provider (optional, auto-detected) */
  embedding_provider?: string;
  /** New embedding dimension (optional, auto-detected) */
  embedding_dimension?: number;
  /** Force rebuild even if config unchanged */
  force?: boolean;
}

/**
 * Response from rebuild embeddings operation.
 */
export interface RebuildEmbeddingsResponse {
  workspace_id: string;
  status: string;
  documents_to_process: number;
  /** Total number of chunks across all documents to be re-embedded */
  chunks_to_process: number;
  vectors_cleared: number;
  embedding_model: string;
  embedding_provider: string;
  embedding_dimension: number;
  /** Model's context length (max input tokens). REQ-25 */
  model_context_length: number;
  estimated_time_seconds?: number;
  job_id?: string;
  /** Warning if chunk size exceeds model context length. REQ-25 */
  compatibility_warning?: string;
}

/**
 * Rebuild workspace embeddings with a new model.
 *
 * This clears all vector embeddings and optionally updates the embedding model.
 * Documents will need to be re-ingested to regenerate embeddings.
 *
 * @implements SPEC-032: Vector database rebuild on embedding model change
 *
 * @param workspaceId - Workspace ID
 * @param request - Rebuild configuration
 * @returns Rebuild status response
 */
export async function rebuildEmbeddings(
  workspaceId: string,
  request: RebuildEmbeddingsRequest,
): Promise<RebuildEmbeddingsResponse> {
  return api.post<RebuildEmbeddingsResponse>(
    `/workspaces/${workspaceId}/rebuild-embeddings`,
    request,
  );
}

// ============================================================================
// Rebuild Knowledge Graph (OODA 256-280)
// ============================================================================

/**
 * Request to rebuild workspace knowledge graph.
 */
export interface RebuildKnowledgeGraphRequest {
  /** New LLM model (optional, keeps current if not provided) */
  llm_model?: string;
  /** New LLM provider (optional, auto-detected) */
  llm_provider?: string;
  /** Force rebuild even if config unchanged */
  force?: boolean;
  /** Whether to also rebuild embeddings (default: false) */
  rebuild_embeddings?: boolean;
}

/**
 * Response from rebuild knowledge graph operation.
 */
export interface RebuildKnowledgeGraphResponse {
  workspace_id: string;
  status: string;
  nodes_cleared: number;
  edges_cleared: number;
  vectors_cleared: number;
  documents_to_process: number;
  /** Total number of chunks across all documents to be reprocessed */
  chunks_to_process: number;
  llm_model: string;
  llm_provider: string;
  estimated_time_seconds?: number;
  track_id?: string;
}

/**
 * Rebuild workspace knowledge graph with a new LLM model.
 *
 * This clears all graph data (entities and relationships) and optionally
 * updates the LLM model. Documents will need to be re-ingested to regenerate
 * the knowledge graph.
 *
 * @implements OODA 256-280: Workspace-scoped rebuild endpoints
 *
 * @param workspaceId - Workspace ID
 * @param request - Rebuild configuration
 * @returns Rebuild status response
 */
export async function rebuildKnowledgeGraph(
  workspaceId: string,
  request: RebuildKnowledgeGraphRequest,
): Promise<RebuildKnowledgeGraphResponse> {
  return api.post<RebuildKnowledgeGraphResponse>(
    `/workspaces/${workspaceId}/rebuild-knowledge-graph`,
    request,
  );
}

// ============================================================================
// Reprocess All Documents (SPEC-032 Focus Area 5)
// ============================================================================

/**
 * Request to reprocess all documents in a workspace.
 */
export interface ReprocessAllRequest {
  /** Whether to include completed documents (default: true) */
  include_completed?: boolean;
  /** Maximum documents to process (default: 1000) */
  max_documents?: number;
}

/**
 * Response from reprocess all documents operation.
 */
export interface ReprocessAllResponse {
  /** Track ID for monitoring progress */
  track_id: string;
  /** Workspace ID */
  workspace_id: string;
  /** Status: "processing" or "no_documents" */
  status: string;
  /** Total documents found */
  documents_found: number;
  /** Documents queued for processing */
  documents_queued: number;
  /** Documents skipped */
  documents_skipped: number;
  /** Estimated time in seconds */
  estimated_time_seconds?: number;
}

/**
 * Reprocess all documents in a workspace.
 *
 * This queues all documents for re-embedding, typically used after
 * a rebuild-embeddings operation. Progress can be monitored via
 * the pipeline status endpoint.
 *
 * @implements SPEC-032: Focus Area 5 - Rebuild with progress
 *
 * @param workspaceId - Workspace ID
 * @param request - Reprocess configuration
 * @returns Reprocess status response
 */
export async function reprocessAllDocuments(
  workspaceId: string,
  request: ReprocessAllRequest = {},
): Promise<ReprocessAllResponse> {
  return api.post<ReprocessAllResponse>(
    `/workspaces/${workspaceId}/reprocess-documents`,
    request,
  );
}

// ============================================================================
// Documents
// ============================================================================

/** Extended paginated response that includes status_counts from the server. */
export interface DocumentsListResult extends PaginatedResponse<Document> {
  status_counts: DocumentStatusCounts;
}

export async function getDocuments(
  params?: PaginationParams & { status?: string },
): Promise<DocumentsListResult> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.sort_by) searchParams.set("sort_by", params.sort_by);
  if (params?.sort_order) searchParams.set("sort_order", params.sort_order);
  if (params?.status) searchParams.set("status", params.status);

  const query = searchParams.toString();

  // API now returns { documents: [...], total, page, page_size, total_pages, has_more, status_counts }
  const response = await api.get<ListDocumentsResponse>(
    `/documents${query ? `?${query}` : ""}`,
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
  return api.get<Document>(`/documents/${documentId}`);
}

export async function uploadDocument(
  data: UploadDocumentRequest,
): Promise<UploadDocumentResponse> {
  return api.post<UploadDocumentResponse>("/documents", data);
}

export async function uploadFile(file: File): Promise<UploadDocumentResponse> {
  const formData = new FormData();
  formData.append("file", file);

  return api.post<UploadDocumentResponse>("/documents/upload", formData, {
    headers: {
      // Let browser set Content-Type with boundary for multipart
    },
  });
}

/**
 * Upload a PDF document for vision-based extraction.
 * @param file The PDF file to upload
 * @param options Upload options (vision settings, title, metadata)
 * @returns PDF upload response with processing status
 */
export async function uploadPdfDocument(
  file: File,
  options?: PdfUploadOptions,
): Promise<PdfUploadResponse> {
  const formData = new FormData();
  formData.append("file", file);

  // Add optional parameters as form fields
  if (options?.enable_vision !== undefined) {
    formData.append("enable_vision", String(options.enable_vision));
  }
  if (options?.vision_provider) {
    formData.append("vision_provider", options.vision_provider);
  }
  if (options?.vision_model) {
    formData.append("vision_model", options.vision_model);
  }
  if (options?.title) {
    formData.append("title", options.title);
  }
  if (options?.metadata) {
    formData.append("metadata", JSON.stringify(options.metadata));
  }
  if (options?.track_id) {
    formData.append("track_id", options.track_id);
  }
  if (options?.force_reindex !== undefined) {
    formData.append("force_reindex", String(options.force_reindex));
  }

  return api.post<PdfUploadResponse>("/documents/pdf", formData, {
    headers: {
      // Let browser set Content-Type with boundary for multipart
    },
  });
}

// ============================================================================
// OODA-19: PDF Progress, Retry, Cancel API Functions
// ============================================================================

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

// ============================================================================
// PDF Viewer API (SPEC-002)
// ============================================================================

/**
 * PDF content response with metadata and extracted markdown.
 *
 * @implements SPEC-002: Document Viewer
 */
export interface PdfContentResponse {
  /** PDF ID */
  pdf_id: string;
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
  // WHY: Use SERVER_BASE_URL (derived from NEXT_PUBLIC_API_URL) for consistency
  // with the rest of the API client. Fixes #79.
  const baseUrl = SERVER_BASE_URL || '';
  return `${baseUrl}/api/v1/documents/pdf/${pdfId}/download`;
}

export async function deleteDocument(documentId: string): Promise<void> {
  return api.delete<void>(`/documents/${documentId}`);
}

export async function deleteAllDocuments(): Promise<{ deleted_count: number }> {
  return api.delete<{ deleted_count: number }>("/documents");
}

/**
 * Reprocess a single document by its document ID.
 * Uses the reprocess endpoint with document_id filter and force flag.
 * @param documentId The ID of the document to reprocess
 * @param force Whether to force reprocess even if document is not failed (default: true)
 */
export async function reprocessDocument(
  documentId: string,
  force: boolean = true,
): Promise<{ track_id: string; message: string; count: number }> {
  return api.post<{ track_id: string; message: string; count: number }>(
    "/documents/reprocess",
    { document_id: documentId, force, max_documents: 1 },
  );
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
 * Response from reprocess failed documents endpoint.
 *
 * @implements OODA-37 - Fixed response type to match backend ReprocessFailedResponse
 */
export interface ReprocessFailedResponse {
  /** Track ID for the reprocess batch */
  track_id: string;
  /** Number of failed documents found */
  failed_found: number;
  /** Number of documents queued for reprocessing */
  requeued: number;
  /** List of document IDs being reprocessed */
  document_ids: string[];
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

// ============================================================================
// Chunk Retry (OODA-03)
// ============================================================================

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

// ============================================================================
// Query
// ============================================================================

export async function query(request: QueryRequest): Promise<QueryResponse> {
  return api.post<QueryResponse>("/query", request);
}

export async function* queryStream(
  request: QueryRequest,
): AsyncGenerator<QueryStreamChunk, void, unknown> {
  yield* streamClient<QueryStreamChunk>("/query/stream", {
    method: "POST",
    body: JSON.stringify({ ...request, stream: true }),
  });
}

// ============================================================================
// Knowledge Graph
// ============================================================================

/**
 * Options for fetching the knowledge graph.
 * Supports server-side filtering for 100k+ node graphs.
 */
export interface GetGraphOptions {
  /** Maximum number of nodes to return (default: 500) */
  limit?: number;
  /** Explicit max_nodes parameter (takes precedence over limit) */
  maxNodes?: number;
  /** Maximum traversal depth from start_node (default: 2) */
  depth?: number;
  /** Focus on a specific node and its neighborhood */
  startNode?: string;
  /** Filter by entity types */
  entity_types?: string[];
  /** Include orphan nodes with no connections */
  include_orphans?: boolean;
}

export async function getGraph(
  options?: GetGraphOptions,
): Promise<KnowledgeGraph> {
  const searchParams = new URLSearchParams();

  // Support both limit and maxNodes (maxNodes takes precedence)
  const nodeLimit = options?.maxNodes ?? options?.limit;
  if (nodeLimit) searchParams.set("max_nodes", String(nodeLimit));

  if (options?.depth) searchParams.set("depth", String(options.depth));
  if (options?.startNode) searchParams.set("start_node", options.startNode);
  if (options?.entity_types)
    searchParams.set("entity_types", options.entity_types.join(","));
  if (options?.include_orphans !== undefined) {
    searchParams.set("include_orphans", String(options.include_orphans));
  }

  const query = searchParams.toString();
  return api.get<KnowledgeGraph>(`/graph${query ? `?${query}` : ""}`);
}

export async function getGraphLabels(): Promise<{
  entity_types: string[];
  relationship_types: string[];
}> {
  return api.get<{ entity_types: string[]; relationship_types: string[] }>(
    "/graph/labels",
  );
}

export async function getGraphStats(): Promise<{
  node_count: number;
  edge_count: number;
  entity_type_counts: Record<string, number>;
  relationship_type_counts: Record<string, number>;
}> {
  return api.get<{
    node_count: number;
    edge_count: number;
    entity_type_counts: Record<string, number>;
    relationship_type_counts: Record<string, number>;
  }>("/graph/stats");
}

/**
 * Search for labels/entities by query string.
 * Used for autocomplete in label search.
 */
export async function searchLabels(
  query: string,
  limit = 20,
): Promise<{ labels: string[] }> {
  return api.get<{ labels: string[] }>(
    `/graph/labels/search?q=${encodeURIComponent(query)}&limit=${limit}`,
  );
}

/**
 * Parameters for full node search.
 */
export interface SearchNodesParams {
  q: string;
  limit?: number;
  includeNeighbors?: boolean;
  neighborDepth?: number;
  entityType?: string;
}

/**
 * Response from full node search.
 */
export interface SearchNodesResponse {
  nodes: GraphNode[];
  edges: GraphEdge[];
  total_matches: number;
  is_truncated: boolean;
}

/**
 * Search for nodes with full data (label and description search).
 * Returns matching nodes with their degrees and connecting edges.
 * Supports optional neighbor inclusion for graph visualization.
 */
export async function searchNodes(
  params: SearchNodesParams,
): Promise<SearchNodesResponse> {
  const urlParams = new URLSearchParams();
  urlParams.set("q", params.q);
  if (params.limit) urlParams.set("limit", String(params.limit));
  if (params.includeNeighbors !== undefined)
    urlParams.set("include_neighbors", String(params.includeNeighbors));
  if (params.neighborDepth !== undefined)
    urlParams.set("neighbor_depth", String(params.neighborDepth));
  if (params.entityType) urlParams.set("entity_type", params.entityType);

  return api.get<SearchNodesResponse>(`/graph/nodes/search?${urlParams}`);
}

/**
 * Popular label with metadata.
 */
export interface PopularLabel {
  label: string;
  entity_type: string;
  degree: number;
  description: string;
}

/**
 * Get popular entities/labels sorted by connection count.
 * Useful for quick access to high-connectivity nodes.
 */
export async function getPopularLabels(options?: {
  limit?: number;
  minDegree?: number;
  entityType?: string;
}): Promise<{ labels: PopularLabel[]; total_entities: number }> {
  const params = new URLSearchParams();
  if (options?.limit) params.set("limit", String(options.limit));
  if (options?.minDegree) params.set("min_degree", String(options.minDegree));
  if (options?.entityType) params.set("entity_type", options.entityType);
  const query = params.toString();
  return api.get(`/graph/labels/popular${query ? `?${query}` : ""}`);
}

// ============================================================================
// Graph Streaming (SSE)
// ============================================================================

/**
 * Metadata sent at the start of graph streaming.
 */
export interface GraphStreamMetadata {
  total_nodes: number;
  total_edges: number;
  nodes_to_stream: number;
  edges_to_stream: number;
}

/**
 * Statistics sent at the end of graph streaming.
 */
export interface GraphStreamStats {
  nodes_count: number;
  edges_count: number;
  duration_ms: number;
}

/**
 * SSE events emitted during graph streaming.
 * Events are sent in order: metadata → nodes (batches) → edges → done
 */
export type GraphStreamEvent =
  | {
      type: "metadata";
      total_nodes: number;
      total_edges: number;
      nodes_to_stream: number;
      edges_to_stream: number;
    }
  | { type: "nodes"; batch: number; total_batches: number; nodes: GraphNode[] }
  | { type: "edges"; edges: GraphEdge[] }
  | {
      type: "done";
      nodes_count: number;
      edges_count: number;
      duration_ms: number;
    }
  | { type: "error"; message: string };

/**
 * Options for streaming graph fetch.
 */
export interface GetGraphStreamOptions {
  /** Maximum nodes to stream (default: 200) */
  maxNodes?: number;
  /** Nodes per batch (default: 50) */
  batchSize?: number;
  /** Focus on specific node neighborhood */
  startNode?: string;
}

/**
 * Stream graph data progressively via SSE.
 *
 * This function yields events as they arrive from the server:
 * 1. `metadata` - Initial graph statistics
 * 2. `nodes` - Multiple batches of nodes (batch_size per event)
 * 3. `edges` - Edges between streamed nodes
 * 4. `done` - Completion summary with timing
 *
 * @example
 * ```typescript
 * for await (const event of graphStream({ maxNodes: 200 })) {
 *   switch (event.type) {
 *     case 'metadata':
 *       console.log(`Streaming ${event.nodes_to_stream} nodes`);
 *       break;
 *     case 'nodes':
 *       console.log(`Batch ${event.batch}/${event.total_batches}`);
 *       addNodesToGraph(event.nodes);
 *       break;
 *     case 'edges':
 *       setEdges(event.edges);
 *       break;
 *     case 'done':
 *       console.log(`Completed in ${event.duration_ms}ms`);
 *       break;
 *   }
 * }
 * ```
 */
export async function* graphStream(
  options?: GetGraphStreamOptions,
): AsyncGenerator<GraphStreamEvent, void, unknown> {
  const searchParams = new URLSearchParams();
  if (options?.maxNodes)
    searchParams.set("max_nodes", String(options.maxNodes));
  if (options?.batchSize)
    searchParams.set("batch_size", String(options.batchSize));
  if (options?.startNode) searchParams.set("start_node", options.startNode);

  const query = searchParams.toString();
  yield* streamClient<GraphStreamEvent>(
    `/graph/stream${query ? `?${query}` : ""}`,
    {
      method: "GET",
    },
  );
}

// ============================================================================
// Entities
// ============================================================================

export async function getEntities(
  params?: PaginationParams & { entity_type?: string; search?: string },
): Promise<PaginatedResponse<Entity>> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.entity_type) searchParams.set("entity_type", params.entity_type);
  if (params?.search) searchParams.set("search", params.search);

  const query = searchParams.toString();
  return api.get<PaginatedResponse<Entity>>(
    `/graph/entities${query ? `?${query}` : ""}`,
  );
}

export async function getEntity(entityId: string): Promise<Entity> {
  return api.get<Entity>(`/graph/entities/${entityId}`);
}

export async function updateEntity(
  entityId: string,
  data: Partial<Entity>,
): Promise<Entity> {
  return api.put<Entity>(`/graph/entities/${entityId}`, data);
}

export async function deleteEntity(entityId: string): Promise<void> {
  return api.delete<void>(`/graph/entities/${entityId}`);
}

export async function mergeEntities(
  request: MergeEntitiesRequest,
): Promise<MergeEntitiesResponse> {
  return api.post<MergeEntitiesResponse>("/graph/entities/merge", request);
}

export async function getEntityNeighborhood(
  entityId: string,
  depth?: number,
): Promise<{ nodes: GraphNode[]; edges: GraphEdge[] }> {
  const query = depth ? `?depth=${depth}` : "";
  return api.get<{ nodes: GraphNode[]; edges: GraphEdge[] }>(
    `/graph/entities/${entityId}/neighborhood${query}`,
  );
}

// ============================================================================
// Relationships
// ============================================================================

export async function getRelationships(
  params?: PaginationParams & { relationship_type?: string },
): Promise<PaginatedResponse<Relationship>> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.relationship_type)
    searchParams.set("relationship_type", params.relationship_type);

  const query = searchParams.toString();
  return api.get<PaginatedResponse<Relationship>>(
    `/graph/relationships${query ? `?${query}` : ""}`,
  );
}

export async function getRelationship(
  relationshipId: string,
): Promise<Relationship> {
  return api.get<Relationship>(`/graph/relationships/${relationshipId}`);
}

export async function updateRelationship(
  relationshipId: string,
  data: Partial<Relationship>,
): Promise<Relationship> {
  return api.put<Relationship>(`/graph/relationships/${relationshipId}`, data);
}

export async function deleteRelationship(
  relationshipId: string,
): Promise<void> {
  return api.delete<void>(`/graph/relationships/${relationshipId}`);
}

// ============================================================================
// Pipeline / Tasks
// ============================================================================

export async function getTasksList(params?: {
  tenant_id?: string;
  workspace_id?: string;
  status?: string;
  task_type?: string;
  page?: number;
  page_size?: number;
}): Promise<import("@/types").TaskListResponse> {
  const searchParams = new URLSearchParams();
  // CRITICAL: Include tenant_id and workspace_id for multi-tenancy isolation
  if (params?.tenant_id) searchParams.set("tenant_id", params.tenant_id);
  if (params?.workspace_id)
    searchParams.set("workspace_id", params.workspace_id);
  if (params?.status) searchParams.set("status", params.status);
  if (params?.task_type) searchParams.set("task_type", params.task_type);
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));

  const query = searchParams.toString();
  return api.get<import("@/types").TaskListResponse>(
    `/tasks${query ? `?${query}` : ""}`,
  );
}

export async function getPipelineStatus(
  tenant_id?: string,
  workspace_id?: string,
): Promise<PipelineStatus> {
  try {
    // Use the tasks list endpoint to derive pipeline status
    // CRITICAL: Pass tenant_id and workspace_id for proper isolation
    const result = await getTasksList({
      tenant_id,
      workspace_id,
      page_size: 50,
    });

    return {
      is_busy: result.statistics.processing > 0,
      running_tasks: result.statistics.processing,
      queued_tasks: result.statistics.pending,
      completed_tasks: result.statistics.indexed,
      failed_tasks: result.statistics.failed,
      tasks: result.tasks,
      statistics: result.statistics,
    };
  } catch (error) {
    console.error("[getPipelineStatus] Error:", error);
    // Return empty status if endpoint fails
    return {
      is_busy: false,
      running_tasks: 0,
      queued_tasks: 0,
      completed_tasks: 0,
      failed_tasks: 0,
      tasks: [],
    };
  }
}

export async function cancelPipeline(): Promise<void> {
  // Cancel all processing tasks
  const result = await getTasksList({ status: "processing" });
  for (const task of result.tasks) {
    await cancelTask(task.track_id);
  }
}

export async function getTaskStatus(
  taskId: string,
): Promise<import("@/types").TaskResponse> {
  return api.get<import("@/types").TaskResponse>(`/tasks/${taskId}`);
}

export async function cancelTask(taskId: string): Promise<void> {
  return api.post<void>(`/tasks/${taskId}/cancel`);
}

export async function retryTask(
  taskId: string,
): Promise<import("@/types").TaskResponse> {
  return api.post<import("@/types").TaskResponse>(`/tasks/${taskId}/retry`);
}

// ============================================================================
// Track Status (Phase 2)
// ============================================================================

/**
 * Get track status by track ID.
 * Returns all documents uploaded with a specific track_id, along with status summary.
 */
export async function getTrackStatus(
  trackId: string,
): Promise<TrackStatusResponse> {
  return api.get<TrackStatusResponse>(`/documents/track/${trackId}`);
}

// ============================================================================
// Ingestion Progress (WebUI Spec WEBUI-005)
// ============================================================================

/**
 * Get real-time progress for a specific track ID.
 * Used as fallback when WebSocket is unavailable.
 */
export interface TrackProgressResponse {
  track_id: string;
  document_id: string;
  document_name: string;
  status: import("@/types/ingestion").IngestionStatus;
  progress: import("@/types/ingestion").ProgressDetail;
  started_at?: string;
  updated_at?: string;
  completed_at?: string;
}

export async function getTrackProgress(
  trackId: string,
): Promise<TrackProgressResponse> {
  return api.get<TrackProgressResponse>(`/ingestion/${trackId}/progress`);
}

/**
 * Get progress for multiple tracks at once.
 */
export async function getMultipleTrackProgress(
  trackIds: string[],
): Promise<TrackProgressResponse[]> {
  return api.post<TrackProgressResponse[]>("/ingestion/progress", {
    track_ids: trackIds,
  });
}

// ============================================================================
// Lineage API (WebUI Spec WEBUI-006)
// ============================================================================

/**
 * Get document lineage from the graph-based lineage endpoint.
 * Uses /lineage/documents/:id which returns entity/relationship summaries.
 */
export async function getDocumentLineage(
  documentId: string,
): Promise<import("@/types/lineage").DocumentLineageResponse> {
  return api.get<import("@/types/lineage").DocumentLineageResponse>(
    `/lineage/documents/${documentId}`,
  );
}

/**
 * Get complete document lineage from persisted KV storage (OODA-07).
 * Uses /documents/:id/lineage which returns full DocumentLineage tree.
 * @implements F5 - Single API call retrieves complete lineage tree
 */
export async function getDocumentFullLineage(
  documentId: string,
): Promise<import("@/types/lineage").DocumentFullLineageResponse> {
  return api.get<import("@/types/lineage").DocumentFullLineageResponse>(
    `/documents/${documentId}/lineage`,
  );
}

/**
 * Get document metadata (all fields in a single response).
 * OODA-11: New endpoint from OODA-07.
 * @implements F1 - All document metadata retrievable via API
 */
export async function getDocumentMetadata(
  documentId: string,
): Promise<Record<string, unknown>> {
  return api.get<Record<string, unknown>>(`/documents/${documentId}/metadata`);
}

/**
 * Get chunk detail including entities and relationships extracted from it.
 */
export async function getChunkDetail(
  chunkId: string,
): Promise<import("@/types/lineage").ChunkDetail> {
  return api.get<import("@/types/lineage").ChunkDetail>(`/chunks/${chunkId}`);
}

/**
 * Get entity provenance showing which chunks contributed to an entity.
 */
export async function getEntityProvenance(
  entityId: string,
): Promise<import("@/types/lineage").EntityProvenanceResponse> {
  return api.get<import("@/types/lineage").EntityProvenanceResponse>(
    `/entities/${entityId}/provenance`,
  );
}

/**
 * Get lineage for a specific chunk.
 * OODA-11: Updated to use ChunkLineageApiResponse from OODA-08.
 */
export async function getChunkLineage(
  chunkId: string,
): Promise<import("@/types/lineage").ChunkLineageApiResponse> {
  return api.get<import("@/types/lineage").ChunkLineageApiResponse>(
    `/chunks/${chunkId}/lineage`,
  );
}

/**
 * Export document lineage as JSON or CSV file.
 * OODA-24: Triggers browser download of lineage data.
 * @implements F5 - Single API call retrieves complete lineage tree
 */
export async function exportDocumentLineage(
  documentId: string,
  format: "json" | "csv" = "json",
): Promise<void> {
  // WHY: Use SERVER_BASE_URL (derived from NEXT_PUBLIC_API_URL) for consistency
  // with the rest of the API client. Fixes #79.
  const baseUrl = SERVER_BASE_URL || '';
  const url = `${baseUrl}/api/v1/documents/${documentId}/lineage/export?format=${format}`;
  // WHY: Create temporary link for download — the endpoint returns
  // Content-Disposition: attachment headers that trigger browser download.
  const link = globalThis.document.createElement("a");
  link.href = url;
  link.download = `${documentId}-lineage.${format}`;
  globalThis.document.body.appendChild(link);
  link.click();
  globalThis.document.body.removeChild(link);
}

// ============================================================================
// Cost API (WebUI Spec WEBUI-007)
// ============================================================================

/**
 * Get cost summary for the current workspace.
 */
export async function getWorkspaceCostSummary(): Promise<
  import("@/types/cost").CostSummary
> {
  return api.get<import("@/types/cost").CostSummary>("/costs/summary");
}

/**
 * Get detailed cost breakdown for a specific document.
 */
export async function getDocumentCost(
  documentId: string,
): Promise<import("@/types/cost").CostBreakdown> {
  return api.get<import("@/types/cost").CostBreakdown>(
    `/documents/${documentId}/cost`,
  );
}

/**
 * Get cost breakdown for a specific ingestion track.
 */
export async function getIngestionCost(
  trackId: string,
): Promise<import("@/types/cost").CostBreakdown> {
  return api.get<import("@/types/cost").CostBreakdown>(
    `/ingestion/${trackId}/cost`,
  );
}

/**
 * Get budget status and limits.
 */
export async function getBudgetStatus(): Promise<
  import("@/types/cost").BudgetInfo
> {
  return api.get<import("@/types/cost").BudgetInfo>("/costs/budget");
}

/**
 * Update budget limits.
 */
export async function updateBudget(
  budget: Partial<import("@/types/cost").BudgetInfo>,
): Promise<import("@/types/cost").BudgetInfo> {
  return api.patch<import("@/types/cost").BudgetInfo>("/costs/budget", budget);
}

/**
 * Get cost history for a time period.
 */
export interface CostHistoryParams {
  start_date?: string;
  end_date?: string;
  granularity?: "hour" | "day" | "week" | "month";
}

export interface CostHistoryPoint {
  timestamp: string;
  total_cost: number;
  total_tokens: number;
  document_count: number;
}

export async function getCostHistory(
  params?: CostHistoryParams,
): Promise<CostHistoryPoint[]> {
  const searchParams = new URLSearchParams();
  if (params?.start_date) searchParams.set("start_date", params.start_date);
  if (params?.end_date) searchParams.set("end_date", params.end_date);
  if (params?.granularity) searchParams.set("granularity", params.granularity);

  const query = searchParams.toString();
  return api.get<CostHistoryPoint[]>(
    `/costs/history${query ? `?${query}` : ""}`,
  );
}

// ============================================================================
// Enhanced Pipeline Status (Phase 3)
// ============================================================================

/**
 * Get enhanced pipeline status with history messages.
 * Falls back to basic status if enhanced endpoint not available.
 *
 * @param tenant_id - Tenant ID for isolation (optional, will use current context if not provided)
 * @param workspace_id - Workspace ID for isolation (optional, will use current context if not provided)
 *
 * CRITICAL: Always pass tenant_id and workspace_id to ensure multi-tenancy isolation
 */
export async function getEnhancedPipelineStatus(
  tenant_id?: string,
  workspace_id?: string,
): Promise<EnhancedPipelineStatus> {
  try {
    // Try enhanced endpoint first with tenant/workspace context
    const params = new URLSearchParams();
    if (tenant_id) params.append("tenant_id", tenant_id);
    if (workspace_id) params.append("workspace_id", workspace_id);
    const query = params.toString();

    return await api.get<EnhancedPipelineStatus>(
      `/pipeline/status${query ? `?${query}` : ""}`,
    );
  } catch {
    // Fall back to basic status derived from tasks with tenant/workspace filtering
    const result = await getTasksList({
      tenant_id,
      workspace_id,
      page_size: 50,
    });

    return {
      is_busy: result.statistics.processing > 0,
      job_name:
        result.statistics.processing > 0 ? "Processing documents" : undefined,
      job_start: undefined,
      total_documents: 0,
      processed_documents: 0,
      current_batch: 0,
      total_batches: 0,
      latest_message:
        result.statistics.processing > 0
          ? `Processing ${result.statistics.processing} document(s)...`
          : undefined,
      history_messages: [],
      cancellation_requested: false,
      pending_tasks: result.statistics.pending,
      processing_tasks: result.statistics.processing,
      completed_tasks: result.statistics.indexed,
      failed_tasks: result.statistics.failed,
    };
  }
}

/**
 * Request pipeline cancellation.
 */
export async function requestPipelineCancellation(): Promise<{
  status: string;
}> {
  try {
    return await api.post<{ status: string }>("/pipeline/cancel");
  } catch {
    // Fall back to cancelling individual tasks
    await cancelPipeline();
    return { status: "cancellation_requested" };
  }
}

// ============================================================================
// Queue Metrics (OODA-21: Objective B)
// ============================================================================

/**
 * Get queue metrics for task queue visibility.
 *
 * @implements FEAT0570 - Queue metrics API
 * @implements OODA-21 - Queue metrics frontend integration
 * @implements OODA-04 - Multi-tenant isolation for queue metrics
 *
 * Returns worker utilization, throughput, wait times, and queue ETA.
 *
 * **CRITICAL**: This function MUST receive tenant/workspace parameters to ensure
 * queue metrics only show activity for the current workspace. Without these
 * parameters, users could see processing activity from other tenants.
 *
 * @param tenantId - Optional tenant ID for filtering. If not provided, uses header context.
 * @param workspaceId - Optional workspace ID for filtering. If not provided, uses header context.
 */
export async function getQueueMetrics(
  tenantId?: string,
  workspaceId?: string,
): Promise<QueueMetrics> {
  try {
    // Build query params for tenant/workspace isolation
    const params = new URLSearchParams();
    if (tenantId) params.append("tenant_id", tenantId);
    if (workspaceId) params.append("workspace_id", workspaceId);
    const query = params.toString();

    return await api.get<QueueMetrics>(
      `/pipeline/queue-metrics${query ? `?${query}` : ""}`,
    );
  } catch {
    // Return default metrics if endpoint not available
    return {
      pending_count: 0,
      processing_count: 0,
      active_workers: 0,
      max_workers: 1,
      worker_utilization: 0,
      avg_wait_time_seconds: 0,
      max_wait_time_seconds: 0,
      throughput_per_minute: 0,
      estimated_queue_time_seconds: 0,
      rate_limited: false,
      timestamp: new Date().toISOString(),
    };
  }
}

// ============================================================================
// Export default API object
// ============================================================================

export const edgequakeApi = {
  // Health
  checkHealth,

  // Auth
  login,
  logout,
  refreshToken,
  getCurrentUser,

  // Tenants & Workspaces
  getTenants,
  getTenant,
  createTenant,
  getWorkspaces,
  getWorkspace,
  getWorkspaceStats,
  createWorkspace,

  // Documents
  getDocuments,
  getDocument,
  uploadDocument,
  uploadFile,
  deleteDocument,
  deleteAllDocuments,
  reprocessDocument,
  scanDocuments,
  reprocessFailedDocuments,
  retryFailedChunks,
  listFailedChunks,

  // Query
  query,
  queryStream,

  // Graph
  getGraph,
  getGraphLabels,
  getGraphStats,
  searchLabels,
  searchNodes,
  getPopularLabels,
  graphStream,

  // Entities
  getEntities,
  getEntity,
  updateEntity,
  deleteEntity,
  mergeEntities,
  getEntityNeighborhood,

  // Relationships
  getRelationships,
  getRelationship,
  updateRelationship,
  deleteRelationship,

  // Pipeline / Tasks
  getPipelineStatus,
  cancelPipeline,
  getTasksList,
  getTaskStatus,
  cancelTask,
  retryTask,

  // Track Status (Phase 2)
  getTrackStatus,

  // Enhanced Pipeline (Phase 3)
  getEnhancedPipelineStatus,
  requestPipelineCancellation,

  // Queue Metrics (OODA-21: Objective B)
  getQueueMetrics,

  // Ingestion Progress (WebUI Spec WEBUI-005)
  getTrackProgress,
  getMultipleTrackProgress,

  // Lineage API (WebUI Spec WEBUI-006)
  getDocumentLineage,
  getDocumentFullLineage,
  getDocumentMetadata,
  getChunkDetail,
  getEntityProvenance,
  getChunkLineage,
  exportDocumentLineage,

  // Cost API (WebUI Spec WEBUI-007)
  getWorkspaceCostSummary,
  getDocumentCost,
  getIngestionCost,
  getBudgetStatus,
  updateBudget,
  getCostHistory,
};

export default edgequakeApi;

// ============================================================================
// Re-export Conversations API
// ============================================================================

export * from "./conversations";
export * from "./folders";
export * from "./query-keys";
