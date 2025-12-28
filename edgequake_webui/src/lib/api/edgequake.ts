import type {
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
  PipelineStatus,
  QueryRequest,
  QueryResponse,
  QueryStreamChunk,
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
  refreshToken: string
): Promise<{ access_token: string; refresh_token: string }> {
  return api.post<{ access_token: string; refresh_token: string }>(
    "/auth/refresh",
    { refresh_token: refreshToken }
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
  chunk_count: number;
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

export async function createTenant(data: {
  name: string;
  description?: string;
}): Promise<Tenant> {
  return api.post<Tenant>("/tenants", data);
}

export async function getWorkspaces(tenantId: string): Promise<Workspace[]> {
  const response = await api.get<WorkspaceListResponse | Workspace[]>(
    `/tenants/${tenantId}/workspaces`
  );
  // Handle both paginated response and legacy array format
  if (Array.isArray(response)) {
    return response;
  }
  return response.items || [];
}

export async function getWorkspace(
  tenantId: string,
  workspaceId: string
): Promise<Workspace> {
  return api.get<Workspace>(`/tenants/${tenantId}/workspaces/${workspaceId}`);
}

export async function getWorkspaceStats(
  workspaceId: string
): Promise<WorkspaceStats> {
  return api.get<WorkspaceStats>(`/workspaces/${workspaceId}/stats`);
}

export async function createWorkspace(
  tenantId: string,
  data: { name: string; description?: string }
): Promise<Workspace> {
  return api.post<Workspace>(`/tenants/${tenantId}/workspaces`, data);
}

// ============================================================================
// Documents
// ============================================================================

/** Extended paginated response that includes status_counts from the server. */
export interface DocumentsListResult extends PaginatedResponse<Document> {
  status_counts: DocumentStatusCounts;
}

export async function getDocuments(
  params?: PaginationParams & { status?: string }
): Promise<DocumentsListResult> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.sort_by) searchParams.set("sort_by", params.sort_by);
  if (params?.sort_order) searchParams.set("sort_order", params.sort_order);
  if (params?.status) searchParams.set("status", params.status);

  const query = searchParams.toString();

  // API now returns { documents: [...], total, page, page_size, status_counts }
  const response = await api.get<ListDocumentsResponse>(
    `/documents${query ? `?${query}` : ""}`
  );

  return {
    items: response.documents || [],
    total: response.total || 0,
    page: response.page || 1,
    page_size: response.page_size || 20,
    has_more: response.page * response.page_size < response.total,
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
  data: UploadDocumentRequest
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

export async function deleteDocument(documentId: string): Promise<void> {
  return api.delete<void>(`/documents/${documentId}`);
}

export async function deleteAllDocuments(): Promise<{ deleted_count: number }> {
  return api.delete<{ deleted_count: number }>("/documents");
}

/**
 * Reprocess a single document by its track ID.
 * Uses the batch reprocess endpoint with track_id filter.
 * @param trackId The track_id of the document to reprocess
 */
export async function reprocessDocument(
  trackId: string
): Promise<{ track_id: string; message: string; count: number }> {
  return api.post<{ track_id: string; message: string; count: number }>(
    "/documents/reprocess",
    { track_id: trackId, max_documents: 1 }
  );
}

/**
 * Scan input directory for new documents.
 * Triggers background scanning and processing of new files.
 * @param path Optional path to scan (defaults to configured input directory)
 */
export async function scanDocuments(
  path?: string
): Promise<{ track_id: string; message: string }> {
  return api.post<{ track_id: string; message: string }>(
    "/documents/scan",
    path ? { path } : {}
  );
}

/**
 * Reprocess all failed documents.
 * Retries processing of documents that previously failed.
 */
export async function reprocessFailedDocuments(): Promise<{
  track_id: string;
  message: string;
  count: number;
}> {
  return api.post<{ track_id: string; message: string; count: number }>(
    "/documents/reprocess"
  );
}

// ============================================================================
// Query
// ============================================================================

export async function query(request: QueryRequest): Promise<QueryResponse> {
  return api.post<QueryResponse>("/query", request);
}

export async function* queryStream(
  request: QueryRequest
): AsyncGenerator<QueryStreamChunk, void, unknown> {
  yield* streamClient<QueryStreamChunk>("/query/stream", {
    method: "POST",
    body: JSON.stringify({ ...request, stream: true }),
  });
}

// ============================================================================
// Knowledge Graph
// ============================================================================

export async function getGraph(options?: {
  limit?: number;
  entity_types?: string[];
  include_orphans?: boolean;
}): Promise<KnowledgeGraph> {
  const searchParams = new URLSearchParams();
  if (options?.limit) searchParams.set("limit", String(options.limit));
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
    "/graph/labels"
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

// ============================================================================
// Entities
// ============================================================================

export async function getEntities(
  params?: PaginationParams & { entity_type?: string; search?: string }
): Promise<PaginatedResponse<Entity>> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.entity_type) searchParams.set("entity_type", params.entity_type);
  if (params?.search) searchParams.set("search", params.search);

  const query = searchParams.toString();
  return api.get<PaginatedResponse<Entity>>(
    `/entities${query ? `?${query}` : ""}`
  );
}

export async function getEntity(entityId: string): Promise<Entity> {
  return api.get<Entity>(`/entities/${entityId}`);
}

export async function updateEntity(
  entityId: string,
  data: Partial<Entity>
): Promise<Entity> {
  return api.patch<Entity>(`/entities/${entityId}`, data);
}

export async function deleteEntity(entityId: string): Promise<void> {
  return api.delete<void>(`/entities/${entityId}`);
}

export async function mergeEntities(
  request: MergeEntitiesRequest
): Promise<MergeEntitiesResponse> {
  return api.post<MergeEntitiesResponse>("/entities/merge", request);
}

export async function getEntityNeighborhood(
  entityId: string,
  depth?: number
): Promise<{ nodes: GraphNode[]; edges: GraphEdge[] }> {
  const query = depth ? `?depth=${depth}` : "";
  return api.get<{ nodes: GraphNode[]; edges: GraphEdge[] }>(
    `/entities/${entityId}/neighborhood${query}`
  );
}

// ============================================================================
// Relationships
// ============================================================================

export async function getRelationships(
  params?: PaginationParams & { relationship_type?: string }
): Promise<PaginatedResponse<Relationship>> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.relationship_type)
    searchParams.set("relationship_type", params.relationship_type);

  const query = searchParams.toString();
  return api.get<PaginatedResponse<Relationship>>(
    `/relationships${query ? `?${query}` : ""}`
  );
}

export async function getRelationship(
  relationshipId: string
): Promise<Relationship> {
  return api.get<Relationship>(`/relationships/${relationshipId}`);
}

export async function updateRelationship(
  relationshipId: string,
  data: Partial<Relationship>
): Promise<Relationship> {
  return api.patch<Relationship>(`/relationships/${relationshipId}`, data);
}

export async function deleteRelationship(
  relationshipId: string
): Promise<void> {
  return api.delete<void>(`/relationships/${relationshipId}`);
}

// ============================================================================
// Pipeline / Tasks
// ============================================================================

export async function getTasksList(params?: {
  status?: string;
  task_type?: string;
  page?: number;
  page_size?: number;
}): Promise<import("@/types").TaskListResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set("status", params.status);
  if (params?.task_type) searchParams.set("task_type", params.task_type);
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));

  const query = searchParams.toString();
  return api.get<import("@/types").TaskListResponse>(
    `/tasks${query ? `?${query}` : ""}`
  );
}

export async function getPipelineStatus(): Promise<PipelineStatus> {
  try {
    // Use the tasks list endpoint to derive pipeline status
    const result = await getTasksList({ page_size: 50 });

    return {
      is_busy: result.statistics.processing > 0,
      running_tasks: result.statistics.processing,
      queued_tasks: result.statistics.pending,
      completed_tasks: result.statistics.indexed,
      failed_tasks: result.statistics.failed,
      tasks: result.tasks,
      statistics: result.statistics,
    };
  } catch {
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
  taskId: string
): Promise<import("@/types").TaskResponse> {
  return api.get<import("@/types").TaskResponse>(`/tasks/${taskId}`);
}

export async function cancelTask(taskId: string): Promise<void> {
  return api.post<void>(`/tasks/${taskId}/cancel`);
}

export async function retryTask(
  taskId: string
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
  trackId: string
): Promise<TrackStatusResponse> {
  return api.get<TrackStatusResponse>(`/documents/track/${trackId}`);
}

// ============================================================================
// Enhanced Pipeline Status (Phase 3)
// ============================================================================

/**
 * Get enhanced pipeline status with history messages.
 * Falls back to basic status if enhanced endpoint not available.
 */
export async function getEnhancedPipelineStatus(): Promise<EnhancedPipelineStatus> {
  try {
    // Try enhanced endpoint first
    return await api.get<EnhancedPipelineStatus>("/pipeline/status");
  } catch {
    // Fall back to basic status derived from tasks
    const result = await getTasksList({ page_size: 50 });

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

  // Query
  query,
  queryStream,

  // Graph
  getGraph,
  getGraphLabels,
  getGraphStats,

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
};

export default edgequakeApi;

// ============================================================================
// Re-export Conversations API
// ============================================================================

export * from "./conversations";
export * from "./folders";
export * from "./query-keys";
