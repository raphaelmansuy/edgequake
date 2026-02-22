/**
 * Workspace and tenant types.
 *
 * @module types/workspaces
 * @see edgequake/crates/edgequake-api/src/handlers/workspaces_types.rs
 */

import type { Timestamp } from "./common.js";

// ── Tenants ───────────────────────────────────────────────────

export interface CreateTenantRequest {
  name: string;
  slug?: string;
  description?: string;
  plan?: string;
  /** Default LLM model for new workspaces (e.g., "gemma3:12b", "gpt-4o-mini"). */
  default_llm_model?: string;
  /** Default LLM provider for new workspaces ("openai", "ollama", "lmstudio"). */
  default_llm_provider?: string;
  /** Default embedding model for new workspaces. */
  default_embedding_model?: string;
  /** Default embedding provider for new workspaces. */
  default_embedding_provider?: string;
  /** Default embedding dimension for new workspaces. */
  default_embedding_dimension?: number;
  /** Default vision LLM model for new workspaces (e.g., "gpt-4o"). Used for PDF image extraction. */
  default_vision_llm_model?: string;
  /** Default vision LLM provider for new workspaces ("openai", "ollama"). */
  default_vision_llm_provider?: string;
}

export interface TenantInfo {
  id: string;
  name: string;
  slug: string;
  plan: string;
  is_active: boolean;
  max_workspaces: number;
  default_llm_model: string;
  default_llm_provider: string;
  default_llm_full_id: string;
  default_embedding_model: string;
  default_embedding_provider: string;
  default_embedding_dimension: number;
  default_embedding_full_id: string;
  /** Default vision LLM model (optional – only set when configured). */
  default_vision_llm_model?: string;
  /** Default vision LLM provider (optional – only set when configured). */
  default_vision_llm_provider?: string;
  created_at: Timestamp;
  updated_at: Timestamp;
}

export interface TenantDetail extends TenantInfo {
  workspace_count?: number;
}

export interface TenantResponse extends TenantInfo {}

export interface UpdateTenantRequest {
  name?: string;
  description?: string;
  plan?: string;
  is_active?: boolean;
}

// ── Workspaces ────────────────────────────────────────────────

export interface CreateWorkspaceRequest {
  name: string;
  slug?: string;
  description?: string;
  max_documents?: number;
  /** LLM model for knowledge graph generation (e.g., "gemma3:12b"). */
  llm_model?: string;
  /** LLM provider ("openai", "ollama", "lmstudio"). */
  llm_provider?: string;
  /** Embedding model name (e.g., "text-embedding-3-small"). */
  embedding_model?: string;
  /** Embedding provider ("openai", "ollama", "lmstudio"). */
  embedding_provider?: string;
  /** Embedding vector dimension override. */
  embedding_dimension?: number;
  /** Vision LLM model for PDF image extraction (e.g., "gpt-4o"). Inherits from tenant if not set. */
  vision_llm_model?: string;
  /** Vision LLM provider ("openai", "ollama"). Inherits from tenant if not set. */
  vision_llm_provider?: string;
}

export interface WorkspaceInfo {
  id: string;
  tenant_id: string;
  name: string;
  slug: string;
  description?: string;
  is_active: boolean;
  max_documents?: number;
  llm_model: string;
  llm_provider: string;
  llm_full_id: string;
  embedding_model: string;
  embedding_provider: string;
  embedding_dimension: number;
  embedding_full_id: string;
  /** Vision LLM model (optional – only set when configured or inherited from tenant). */
  vision_llm_model?: string;
  /** Vision LLM provider (optional – only set when configured or inherited from tenant). */
  vision_llm_provider?: string;
  created_at: Timestamp;
  updated_at: Timestamp;
}

export interface WorkspaceDetail extends WorkspaceInfo {
  document_count?: number;
  entity_count?: number;
}

export interface WorkspaceResponse extends WorkspaceInfo {}

export interface UpdateWorkspaceRequest {
  name?: string;
  description?: string;
  is_active?: boolean;
  max_documents?: number;
  llm_model?: string;
  llm_provider?: string;
  embedding_model?: string;
  embedding_provider?: string;
  embedding_dimension?: number;
  /** Vision LLM model for PDF image extraction. */
  vision_llm_model?: string;
  /** Vision LLM provider. */
  vision_llm_provider?: string;
}

export interface WorkspaceStats {
  workspace_id: string;
  document_count: number;
  entity_count: number;
  relationship_count: number;
  chunk_count: number;
  storage_bytes?: number;
}

export interface MetricsHistoryQuery {
  from?: string;
  to?: string;
  interval?: string;
}

export interface MetricsHistory {
  workspace_id: string;
  data_points: Array<{
    timestamp: Timestamp;
    document_count: number;
    entity_count: number;
    relationship_count: number;
  }>;
}
