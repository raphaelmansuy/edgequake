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

/**
 * Get a workspace by its URL-friendly slug.
 * Useful for URL-based workspace routing.
 */
export async function getWorkspaceBySlug(
  tenantId: string,
  slug: string
): Promise<Workspace> {
  return api.get<Workspace>(`/tenants/${tenantId}/workspaces/by-slug/${slug}`);
}

export async function getWorkspaceStats(
  workspaceId: string
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
  data: CreateWorkspaceRequest
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
}

/**
 * Update an existing workspace.
 *
 * @implements SPEC-032: Workspace-level configuration update
 *
 * @param tenantId - Parent tenant ID
 * @param workspaceId - Workspace ID to update
 * @param data - Update request
 * @returns Updated workspace
 */
export async function updateWorkspace(
  tenantId: string,
  workspaceId: string,
  data: UpdateWorkspaceRequest
): Promise<Workspace> {
  return api.patch<Workspace>(`/tenants/${tenantId}/workspaces/${workspaceId}`, data);
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
  vectors_cleared: number;
  embedding_model: string;
  embedding_provider: string;
  embedding_dimension: number;
  estimated_time_seconds?: number;
  job_id?: string;
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
  request: RebuildEmbeddingsRequest
): Promise<RebuildEmbeddingsResponse> {
  return api.post<RebuildEmbeddingsResponse>(
    `/workspaces/${workspaceId}/rebuild-embeddings`,
    request
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
  request: ReprocessAllRequest = {}
): Promise<ReprocessAllResponse> {
  return api.post<ReprocessAllResponse>(
    `/workspaces/${workspaceId}/reprocess-documents`,
    request
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

  // API now returns { documents: [...], total, page, page_size, total_pages, has_more, status_counts }
  const response = await api.get<ListDocumentsResponse>(
    `/documents${query ? `?${query}` : ""}`
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
  options?: GetGraphOptions
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

/**
 * Search for labels/entities by query string.
 * Used for autocomplete in label search.
 */
export async function searchLabels(
  query: string,
  limit = 20
): Promise<{ labels: string[] }> {
  return api.get<{ labels: string[] }>(
    `/graph/labels/search?q=${encodeURIComponent(query)}&limit=${limit}`
  );
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
  options?: GetGraphStreamOptions
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
    }
  );
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
    `/graph/entities${query ? `?${query}` : ""}`
  );
}

export async function getEntity(entityId: string): Promise<Entity> {
  return api.get<Entity>(`/graph/entities/${entityId}`);
}

export async function updateEntity(
  entityId: string,
  data: Partial<Entity>
): Promise<Entity> {
  return api.put<Entity>(`/graph/entities/${entityId}`, data);
}

export async function deleteEntity(entityId: string): Promise<void> {
  return api.delete<void>(`/graph/entities/${entityId}`);
}

export async function mergeEntities(
  request: MergeEntitiesRequest
): Promise<MergeEntitiesResponse> {
  return api.post<MergeEntitiesResponse>("/graph/entities/merge", request);
}

export async function getEntityNeighborhood(
  entityId: string,
  depth?: number
): Promise<{ nodes: GraphNode[]; edges: GraphEdge[] }> {
  const query = depth ? `?depth=${depth}` : "";
  return api.get<{ nodes: GraphNode[]; edges: GraphEdge[] }>(
    `/graph/entities/${entityId}/neighborhood${query}`
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
    `/graph/relationships${query ? `?${query}` : ""}`
  );
}

export async function getRelationship(
  relationshipId: string
): Promise<Relationship> {
  return api.get<Relationship>(`/graph/relationships/${relationshipId}`);
}

export async function updateRelationship(
  relationshipId: string,
  data: Partial<Relationship>
): Promise<Relationship> {
  return api.put<Relationship>(`/graph/relationships/${relationshipId}`, data);
}

export async function deleteRelationship(
  relationshipId: string
): Promise<void> {
  return api.delete<void>(`/graph/relationships/${relationshipId}`);
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
  trackId: string
): Promise<TrackProgressResponse> {
  return api.get<TrackProgressResponse>(`/ingestion/${trackId}/progress`);
}

/**
 * Get progress for multiple tracks at once.
 */
export async function getMultipleTrackProgress(
  trackIds: string[]
): Promise<TrackProgressResponse[]> {
  return api.post<TrackProgressResponse[]>("/ingestion/progress", {
    track_ids: trackIds,
  });
}

// ============================================================================
// Lineage API (WebUI Spec WEBUI-006)
// ============================================================================

/**
 * Get document lineage showing all chunks extracted from a document.
 */
export async function getDocumentLineage(
  documentId: string
): Promise<import("@/types/lineage").DocumentLineageResponse> {
  return api.get<import("@/types/lineage").DocumentLineageResponse>(
    `/documents/${documentId}/lineage`
  );
}

/**
 * Get chunk detail including entities and relationships extracted from it.
 */
export async function getChunkDetail(
  chunkId: string
): Promise<import("@/types/lineage").ChunkDetail> {
  return api.get<import("@/types/lineage").ChunkDetail>(`/chunks/${chunkId}`);
}

/**
 * Get entity provenance showing which chunks contributed to an entity.
 */
export async function getEntityProvenance(
  entityId: string
): Promise<import("@/types/lineage").EntityProvenanceResponse> {
  return api.get<import("@/types/lineage").EntityProvenanceResponse>(
    `/entities/${entityId}/provenance`
  );
}

/**
 * Get lineage for a specific chunk.
 */
export async function getChunkLineage(
  chunkId: string
): Promise<import("@/types/lineage").ChunkLineage> {
  return api.get<import("@/types/lineage").ChunkLineage>(
    `/chunks/${chunkId}/lineage`
  );
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
  documentId: string
): Promise<import("@/types/cost").CostBreakdown> {
  return api.get<import("@/types/cost").CostBreakdown>(
    `/documents/${documentId}/cost`
  );
}

/**
 * Get cost breakdown for a specific ingestion track.
 */
export async function getIngestionCost(
  trackId: string
): Promise<import("@/types/cost").CostBreakdown> {
  return api.get<import("@/types/cost").CostBreakdown>(
    `/ingestion/${trackId}/cost`
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
  budget: Partial<import("@/types/cost").BudgetInfo>
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
  params?: CostHistoryParams
): Promise<CostHistoryPoint[]> {
  const searchParams = new URLSearchParams();
  if (params?.start_date) searchParams.set("start_date", params.start_date);
  if (params?.end_date) searchParams.set("end_date", params.end_date);
  if (params?.granularity) searchParams.set("granularity", params.granularity);

  const query = searchParams.toString();
  return api.get<CostHistoryPoint[]>(
    `/costs/history${query ? `?${query}` : ""}`
  );
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
  searchLabels,
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

  // Ingestion Progress (WebUI Spec WEBUI-005)
  getTrackProgress,
  getMultipleTrackProgress,

  // Lineage API (WebUI Spec WEBUI-006)
  getDocumentLineage,
  getChunkDetail,
  getEntityProvenance,
  getChunkLineage,

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
