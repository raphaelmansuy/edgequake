/**
 * Provider status types
 *
 * @implements SPEC-032: Ollama/LM Studio provider support - WebUI types
 * @iteration OODA Loop #5 - Phase 5E.5
 */

export interface ProviderStatusResponse {
  provider: LLMProviderStatus;
  embedding: EmbeddingProviderStatus;
  storage: StorageStatus;
  metadata: StatusMetadata;
}

export interface LLMProviderStatus {
  name: string;
  type: "llm";
  status: ConnectionStatus;
  model: string;
  base_url?: string;
  config: Record<string, any>;
}

export interface EmbeddingProviderStatus {
  name: string;
  type: "embedding";
  status: ConnectionStatus;
  model: string;
  dimension: number;
}

export interface StorageStatus {
  type: "memory" | "postgres";
  dimension: number;
  dimension_mismatch: boolean;
  namespace: string;
}

export type ConnectionStatus =
  | "connected"
  | "connecting"
  | "disconnected"
  | "error";

export interface StatusMetadata {
  checked_at: string; // ISO 8601
  uptime_seconds: number;
}

// ============================================================================
// Available Providers Registry Types (SPEC-032 OODA #14)
// ============================================================================

/**
 * Response from GET /api/v1/settings/providers
 * Lists all available LLM and embedding providers with their configuration requirements.
 */
export interface AvailableProvidersResponse {
  /** Available LLM providers (chat completion) */
  llm_providers: ProviderInfo[];
  /** Available embedding providers */
  embedding_providers: ProviderInfo[];
  /** Currently active LLM provider name */
  active_llm_provider: string;
  /** Currently active embedding provider name */
  active_embedding_provider: string;
}

/**
 * Information about a single provider.
 */
export interface ProviderInfo {
  /** Provider identifier (e.g., "openai", "ollama", "lmstudio") */
  id: string;
  /** Human-readable name */
  name: string;
  /** Provider description */
  description: string;
  /** Whether this provider is currently available/configured */
  available: boolean;
  /** Required environment variables or configuration */
  config_requirements: ConfigRequirement[];
  /** Default models for this provider */
  default_models: DefaultModels;
}

/**
 * A configuration requirement for a provider.
 */
export interface ConfigRequirement {
  /** Environment variable name */
  name: string;
  /** Whether this configuration is required */
  required: boolean;
  /** Description of what this configuration is for */
  description: string;
}

/**
 * Default model configuration for a provider.
 */
export interface DefaultModels {
  /** Default chat/completion model */
  chat_model: string;
  /** Default embedding model */
  embedding_model: string;
  /** Default embedding dimension */
  embedding_dimension: number;
}
