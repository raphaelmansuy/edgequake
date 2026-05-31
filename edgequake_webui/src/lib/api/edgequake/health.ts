/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { getRuntimeServerBaseUrl } from "@/lib/runtime-config";

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
    WorkspacePdfParserBackendUpdate,
} from "@/types";

export async function checkHealth(): Promise<HealthResponse> {
  const serverBaseUrl = getRuntimeServerBaseUrl();
  const url = serverBaseUrl ? `${serverBaseUrl}/health` : "/health";
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Health check failed: ${response.statusText}`);
  }
  return response.json();
}

export async function checkReady(): Promise<{ status: string }> {
  const serverBaseUrl = getRuntimeServerBaseUrl();
  const url = serverBaseUrl ? `${serverBaseUrl}/ready` : "/ready";
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`Readiness check failed: ${response.statusText}`);
  }
  return response.json();
}
