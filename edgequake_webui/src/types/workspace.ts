/** Tenant and workspace multi-tenancy types. */

import type { PdfParserBackend } from "./graph";

export interface Tenant {
  /** Tenant unique identifier (UUID). */
  id: string;
  /** Tenant display name. */
  name: string;
  /** URL-friendly slug. */
  slug?: string;
  /** Optional description. */
  description?: string;
  /** Subscription plan (free, basic, pro, enterprise). */
  plan?: string;
  /** Whether the tenant is active. */
  is_active?: boolean;
  /** Maximum workspaces allowed for this tenant. */
  max_workspaces?: number;

  // === Default LLM Configuration (SPEC-032) ===

  /**
   * Default LLM model for new workspaces (e.g., "gemma3:12b", "gpt-4o-mini").
   * @implements SPEC-032: Tenant-level LLM configuration defaults
   */
  default_llm_model?: string;
  /**
   * Default LLM provider for new workspaces (e.g., "ollama", "openai", "lmstudio").
   * @implements SPEC-032: Tenant-level LLM configuration defaults
   */
  default_llm_provider?: string;
  /**
   * Fully qualified default LLM model ID (provider/model format).
   * @implements SPEC-032: Combined model ID format
   */
  default_llm_full_id?: string;

  // === Default Embedding Configuration (SPEC-032) ===

  /**
   * Default embedding model for new workspaces (e.g., "text-embedding-3-small").
   * @implements SPEC-032: Tenant-level embedding configuration defaults
   */
  default_embedding_model?: string;
  /**
   * Default embedding provider for new workspaces (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Tenant-level embedding configuration defaults
   */
  default_embedding_provider?: string;
  /**
   * Default embedding dimension for new workspaces (e.g., 1536 for OpenAI, 768 for Ollama).
   * @implements SPEC-032: Tenant-level embedding configuration defaults
   */
  default_embedding_dimension?: number;
  /**
   * Fully qualified default embedding model ID (provider/model format).
   * @implements SPEC-032: Combined model ID format
   */
  default_embedding_full_id?: string;

  // === Default Vision LLM Configuration (SPEC-041) ===

  /**
   * Default vision LLM model for new workspaces (e.g., "gpt-4o", "gemma3:12b").
   * Used for PDF vision extraction. Workspaces inherit this if not overridden.
   * @implements SPEC-041: Tenant-level vision LLM configuration defaults
   */
  default_vision_llm_model?: string;
  /**
   * Default vision LLM provider for new workspaces (e.g., "openai", "ollama").
   * @implements SPEC-041: Tenant-level vision LLM configuration defaults
   */
  default_vision_llm_provider?: string;

  /** Creation timestamp. */
  created_at: string;
  /** Last update timestamp. */
  updated_at?: string;
}

export interface Workspace {
  /** Workspace unique identifier (UUID). */
  id: string;
  /** Parent tenant ID. */
  tenant_id: string;
  /** Workspace display name. */
  name: string;
  /** URL-friendly slug. */
  slug?: string;
  /** Optional description. */
  description?: string;
  /** Whether the workspace is active. */
  is_active?: boolean;
  /** Maximum documents allowed. */
  max_documents?: number;
  /** Number of documents (from stats, may not be returned inline). */
  document_count?: number;
  /** Number of entities (from stats, may not be returned inline). */
  entity_count?: number;
  /**
   * LLM model name for knowledge graph generation and summarization.
   * @implements SPEC-032: Workspace-level LLM configuration
   */
  llm_model?: string;
  /**
   * LLM provider ID (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Workspace-level LLM configuration
   */
  llm_provider?: string;
  /**
   * Fully qualified LLM model ID (provider/model format).
   * @implements SPEC-032: Combined model ID format
   */
  llm_full_id?: string;
  /**
   * Embedding model name (e.g., "text-embedding-3-small").
   * @implements SPEC-032: Workspace-level embedding configuration
   */
  embedding_model?: string;
  /**
   * Embedding provider ID (e.g., "openai", "ollama", "lmstudio").
   * @implements SPEC-032: Workspace-level embedding configuration
   */
  embedding_provider?: string;
  /**
   * Embedding dimension (e.g., 1536 for OpenAI, 768 for Ollama).
   * @implements SPEC-032: Workspace-level embedding configuration
   */
  embedding_dimension?: number;
  /**
   * Fully qualified embedding model ID (provider/model format).
   * @implements SPEC-032: Combined model ID format
   */
  embedding_full_id?: string;
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
  /**
   * Default PDF parser backend for this workspace.
   * @implements Mission 03: Configurable PDF parser backend
   */
  pdf_parser_backend?: PdfParserBackend;
  /**
   * Custom entity types for the extraction pipeline.
   * If absent, the server uses the default types (PERSON, ORGANIZATION, etc.).
   * Surfaced from workspace metadata JSONB.
   * @implements SPEC-085: Custom entity configuration from UI
   */
  entity_types?: string[];
  /**
   * When true, extraction is limited to listed types; unknown types map to OTHER.
   * @implements SPEC-013 entity_extraction strict mode
   */
  entity_types_strict?: boolean;
  /** Creation timestamp. */
  created_at: string;
  /** Last update timestamp. */
  updated_at?: string;
}

/**
 * Request to create a new workspace.
 * @implements SPEC-032: Workspace LLM and embedding configuration on creation
 */
export interface CreateWorkspaceRequest {
  /** Workspace display name. */
  name: string;
  /** URL-friendly slug (optional, auto-generated from name). */
  slug?: string;
  /** Optional description. */
  description?: string;
  /** Maximum documents allowed (optional). */
  max_documents?: number;
  /**
   * LLM model name for knowledge graph generation and summarization.
   * If not provided, uses server default (e.g., "gemma3:12b").
   * Can be a full ID like "ollama/gemma3:12b" for explicit provider.
   * @implements SPEC-032: Workspace-level LLM configuration
   */
  llm_model?: string;
  /**
   * LLM provider ID (e.g., "openai", "ollama", "lmstudio").
   * If not provided, auto-detected from llm_model.
   * @implements SPEC-032: Workspace-level LLM configuration
   */
  llm_provider?: string;
  /**
   * Embedding model name (e.g., "text-embedding-3-small", "embeddinggemma:latest").
   * If not provided, uses server default.
   * Can be a full ID like "openai/text-embedding-3-small" for explicit provider.
   */
  embedding_model?: string;
  /**
   * Embedding provider ID (e.g., "openai", "ollama", "lmstudio").
   * If not provided, auto-detected from embedding_model.
   */
  embedding_provider?: string;
  /**
   * Embedding dimension override.
   * If not provided, auto-detected from embedding_model.
   */
  embedding_dimension?: number;
  /**
   * Vision LLM model for PDF-to-Markdown image extraction (e.g., "gpt-4o", "gemma3:12b").
   * If not provided, inherits from tenant default_vision_llm_model or server default.
   * Must support vision (supports_vision === true).
   * @implements SPEC-041: Workspace-scoped Vision LLM for PDF processing
   */
  vision_llm_model?: string;
  /**
   * Vision LLM provider for PDF-to-Markdown extraction ("openai", "ollama", "lmstudio").
   * If not provided, auto-detected from vision_llm_model.
   * @implements SPEC-041: Workspace-scoped Vision LLM for PDF processing
   */
  vision_llm_provider?: string;
  /** Default PDF parser backend for this workspace. */
  pdf_parser_backend?: PdfParserBackend;
  /**
   * Custom entity types for this workspace's extraction pipeline.
   * Normalized to UPPERCASE_UNDERSCORED and deduplicated (max 50).
   * If not provided, server defaults are used (PERSON, ORGANIZATION, etc.).
   * @implements SPEC-085: Custom entity configuration from UI
   */
  entity_types?: string[];
}

/**
 * Workspace statistics response.
 * @implements SPEC-032: Workspace stats for detail page
 */
export interface WorkspaceStats {
  /** Total number of documents in workspace */
  document_count: number;
  /** Total number of entities extracted */
  entity_count: number;
  /** Total number of relationships */
  relationship_count: number;
  /** Number of distinct entity types (e.g., PERSON, ORGANIZATION, …) */
  entity_type_count?: number;
  /** Total number of text chunks */
  chunk_count: number;
  /** Total number of vectors stored */
  vector_count?: number;
  /** Total characters processed */
  total_characters?: number;
  /** Total tokens used */
  total_tokens?: number;
}
