import type {
  Document,
  Entity,
  GraphEdge,
  GraphNode,
  HealthResponse,
  KnowledgeGraph,
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

export async function getTenants(): Promise<Tenant[]> {
  return api.get<Tenant[]>("/tenants");
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
  return api.get<Workspace[]>(`/tenants/${tenantId}/workspaces`);
}

export async function getWorkspace(
  tenantId: string,
  workspaceId: string
): Promise<Workspace> {
  return api.get<Workspace>(`/tenants/${tenantId}/workspaces/${workspaceId}`);
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

export async function getDocuments(
  params?: PaginationParams & { status?: string }
): Promise<PaginatedResponse<Document>> {
  const searchParams = new URLSearchParams();
  if (params?.page) searchParams.set("page", String(params.page));
  if (params?.page_size)
    searchParams.set("page_size", String(params.page_size));
  if (params?.sort_by) searchParams.set("sort_by", params.sort_by);
  if (params?.sort_order) searchParams.set("sort_order", params.sort_order);
  if (params?.status) searchParams.set("status", params.status);

  const query = searchParams.toString();
  return api.get<PaginatedResponse<Document>>(
    `/documents${query ? `?${query}` : ""}`
  );
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

export async function reprocessDocument(
  documentId: string
): Promise<UploadDocumentResponse> {
  return api.post<UploadDocumentResponse>(`/documents/${documentId}/reprocess`);
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

export async function getPipelineStatus(): Promise<PipelineStatus> {
  return api.get<PipelineStatus>("/pipeline/status");
}

export async function getTaskStatus(
  taskId: string
): Promise<PipelineStatus["tasks"][0]> {
  return api.get<PipelineStatus["tasks"][0]>(`/tasks/${taskId}`);
}

export async function cancelTask(taskId: string): Promise<void> {
  return api.post<void>(`/tasks/${taskId}/cancel`);
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
  createWorkspace,

  // Documents
  getDocuments,
  getDocument,
  uploadDocument,
  uploadFile,
  deleteDocument,
  deleteAllDocuments,
  reprocessDocument,

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

  // Pipeline
  getPipelineStatus,
  getTaskStatus,
  cancelTask,
};

export default edgequakeApi;
