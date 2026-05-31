/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { api } from "../client";

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
