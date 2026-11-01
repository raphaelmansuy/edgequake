/**
 * Domain API module — split from edgequake.ts (SPEC-017 UI-DRY-001).
 */

import { api } from "../client";
import { withoutImmutableProfileFields } from "@/lib/onboarding/rag-profile-payload";
import { fetchAllPages } from "../fetch-all-pages";
import { buildQueryString, withQuery } from "../query-params";

import type {
  CreateWorkspaceRequest,
  Entity,
  Tenant,
  Workspace,
  WorkspacePdfParserBackendUpdate,
} from "@/types";

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

type IdFields = { id?: string; workspace_id?: string };

function unwrapListPage<T>(
  response: { items?: T[]; total?: number } | T[],
): { items: T[]; total: number } {
  if (Array.isArray(response)) {
    return { items: response, total: response.length };
  }
  const items = response.items || [];
  const total =
    typeof response.total === "number" ? response.total : items.length;
  return { items, total };
}

function withStableId<T extends IdFields>(row: T): T & { id: string } {
  const id = row.id || row.workspace_id || "";
  return { ...row, id };
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
  /** True when counts may be outdated (served from cache under load). */
  stale: boolean;
}

export async function getTenants(): Promise<Tenant[]> {
  return fetchAllPages(async (offset, limit) => {
    const response = await api.get<TenantListResponse | Tenant[]>(
      withQuery("/tenants", buildQueryString({ limit, offset })),
    );
    const page = unwrapListPage(response);
    return {
      items: page.items.map((row) => withStableId(row)),
      total: page.total,
    };
  });
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
  /** URL slug. Omitted means it is derived from the name. */
  slug?: string;
  /** SPEC-181: who becomes Owner. Omitted means the calling principal. */
  owner?:
    | { kind: 'self_principal' }
    | { kind: 'user_id'; user_id: string }
    | { kind: 'email'; email: string };
  /** SPEC-181: the workspace the organization starts with. */
  workspace?: { name?: string; slug?: string; rag_profile?: 'graph' | 'normal' };
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

/**
 * Partial tenant update (`PUT /tenants/{id}`). Omitted fields are left unchanged.
 */
export interface UpdateTenantRequest {
  name?: string;
  description?: string;
  /** `#rrggbb` / `#rgb` to set, or `none` (or empty) to clear to the EdgeQuake default. */
  primary_color?: string;
}

/**
 * Update a tenant. Requires tenant manage rights (owner or admin of the tenant).
 */
export async function updateTenant(
  tenantId: string,
  data: UpdateTenantRequest,
): Promise<Tenant> {
  return api.put<Tenant>(`/tenants/${encodeURIComponent(tenantId)}`, data);
}

export async function getWorkspaces(tenantId: string): Promise<Workspace[]> {
  return fetchAllPages(async (offset, limit) => {
    const response = await api.get<WorkspaceListResponse | Workspace[]>(
      withQuery(
        `/tenants/${tenantId}/workspaces`,
        buildQueryString({ limit, offset }),
      ),
    );
    const page = unwrapListPage(response);
    return {
      items: page.items.map((row) => withStableId(row)),
      total: page.total,
    };
  });
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
  /** Default PDF parser backend for this workspace. */
  pdf_parser_backend?: WorkspacePdfParserBackendUpdate;
  /** Entity types for future ingestions (SPEC-085 / GitHub #216). */
  entity_types?: string[];
  /** Strict entity type enforcement (default true). */
  entity_types_strict?: boolean;
  /**
   * Extraction output language (SPEC-096).
   * Pass `""` or `"none"` to clear the workspace override.
   */
  extraction_language?: string | null;
  /**
   * Chunking mode (SPEC-116). Pass `"inherit"` / `"none"` / `""` to clear.
   */
  chunking_mode?: string | null;
  /** Fixed chunk size when mode=fixed (SPEC-116). */
  chunk_token_size?: number | null;
  /** Fixed overlap when mode=fixed (SPEC-116). */
  chunk_overlap_token_size?: number | null;
  /**
   * Custom entity-type → hex color map (SPEC-102).
   * Empty object clears overrides.
   */
  entity_type_colors?: Record<string, string>;
  /** Relation type allow-list (SPEC-114). Empty clears (free-form). */
  relation_types?: string[];
  /** Strict relation type enforcement (SPEC-114). */
  relation_types_strict?: boolean;
  /** Domain preset id (SPEC-114). */
  kg_schema_preset?: string;
  /** Typed edges (SPEC-114b). Empty clears. */
  relation_edges?: Array<{ source: string; relation: string; target: string }>;
  /** SPEC-109: workspace default reasoning effort. */
  default_reasoning_effort?: string;
  /** SPEC-109: merge llm_roles metadata (extract/query effort). */
  llm_roles?: Record<
    string,
    {
      provider?: string | null;
      model?: string | null;
      reasoning_effort?: string | null;
      connection_id?: string | null;
    }
  >;
  /**
   * SPEC-117 extract budget. Pass `"inherit"` / `"none"` / `""` to clear.
   */
  extract_budget_mode?: string | null;
  extract_max_entities?: number | null;
  extract_max_records?: number | null;
  /**
   * SPEC-160 extraction mode. `extraction_mode`, `decision_gate_preset` and
   * `decision_model` accept `"inherit"` to clear; `decision_pack_size: 0` clears.
   */
  extraction_mode?: string;
  decision_gate_preset?: string;
  decision_model?: string;
  decision_pack_size?: number;
  decision_enabled?: boolean;
  /** SPEC-015V — Vision extract toggles (absent → server default ON). */
  vision_extract_images?: boolean;
  vision_extract_charts?: boolean;
  vision_extract_figures?: boolean;
  vision_page_system_prompt?: string;
  vision_image_system_prompt?: string;
  vision_chart_system_prompt?: string;
  vision_figure_system_prompt?: string;
}

/**
 * Update an existing workspace.
 *
 * @implements SPEC-032: Workspace-level configuration update
 *
 * Note: Backend uses PUT /workspaces/{workspace_id} (no tenant prefix).
 * Pass `tenantId` as X-Tenant-ID so the workspace is resolved in the correct
 * tenant (required when creating a tenant then PATCHing its Default Workspace).
 *
 * @param tenantId - Parent tenant ID (sent as X-Tenant-ID)
 * @param workspaceId - Workspace ID to update
 * @param data - Update request
 * @returns Updated workspace
 */
export async function updateWorkspace(
  tenantId: string,
  workspaceId: string,
  data: UpdateWorkspaceRequest,
): Promise<Workspace> {
  // Backend route: PUT /api/v1/workspaces/{workspace_id}
  // SPEC-180: the profile is fixed at create; a caller that spreads a Workspace must not 409.
  return api.put<Workspace>(`/workspaces/${workspaceId}`, withoutImmutableProfileFields(data), {
    headers: {
      "X-Tenant-ID": tenantId,
      "X-Workspace-ID": workspaceId,
    },
  });
}

/**
 * Delete a workspace and cascade delete all associated data.
 *
 * FIX #171: Workspace deletion from UI.
 *
 * @param workspaceId - Workspace ID to delete
 */
/**
 * SPEC-181: deleting a workspace returns a purge receipt (202). Every plane records its
 * outcome, so a failed object-store delete is a retryable `partial`, not a silent leak.
 */
export async function deleteWorkspace(workspaceId: string): Promise<PurgeReceipt> {
  return api.delete<PurgeReceipt>(`/workspaces/${encodeURIComponent(workspaceId)}`);
}

/** What a workspace or organization delete would destroy, before it happens. */
export interface PurgePreview {
  scope_kind: 'workspace' | 'organization';
  scope_id: string;
  name: string;
  workspaces: number;
  documents: number;
  entities: number;
  relationships: number;
  vector_rows: number;
  original_objects: number;
}

export type PlaneOutcome =
  | { status: 'done'; count?: number }
  | { status: 'skipped'; reason: string }
  | { status: 'failed'; error: string };

export interface PurgeReceipt {
  receipt_id: string;
  scope_kind: 'workspace' | 'organization';
  scope_id: string;
  tenant_id: string;
  status: 'running' | 'done' | 'partial' | 'failed';
  planes: Record<string, PlaneOutcome>;
  planes_total: number;
  planes_done: number;
  started_at: string;
  finished_at?: string;
}

export async function getWorkspacePurgePreview(workspaceId: string): Promise<PurgePreview> {
  return api.get<PurgePreview>(`/workspaces/${encodeURIComponent(workspaceId)}/purge-preview`);
}

/** Archive a workspace: it leaves the switcher and stops counting toward the plan. */
export async function archiveWorkspace(
  workspaceId: string,
  reason?: string,
): Promise<Workspace> {
  return api.post<Workspace>(
    `/workspaces/${encodeURIComponent(workspaceId)}/archive`,
    reason ? { reason } : {},
  );
}

/** Restore an archived workspace, subject to the plan's workspace limit. */
export async function restoreWorkspace(workspaceId: string): Promise<Workspace> {
  return api.post<Workspace>(`/workspaces/${encodeURIComponent(workspaceId)}/restore`, {});
}

export async function getTenantPurgePreview(tenantId: string): Promise<PurgePreview> {
  return api.get<PurgePreview>(`/tenants/${encodeURIComponent(tenantId)}/purge-preview`);
}

/** Delete an organization (platform admin, or organization Owner). Returns a receipt. */
export async function deleteTenant(tenantId: string): Promise<PurgeReceipt> {
  return api.delete<PurgeReceipt>(`/tenants/${encodeURIComponent(tenantId)}`);
}

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
