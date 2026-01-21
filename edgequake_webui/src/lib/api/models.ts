/**
 * Models Configuration API Client
 *
 * @implements FEAT0470: Models Configuration API
 * @implements FEAT0471: Provider Capability Exposure
 *
 * Provides functions to fetch model configuration including:
 * - Available providers and their models
 * - Model capabilities (context length, vision, function calling)
 * - Cost information per 1K tokens
 * - Provider health status
 */

import { apiClient } from "./client";

// ============================================================================
// Types
// ============================================================================

/**
 * Model capabilities information.
 */
export interface ModelCapabilities {
  context_length: number;
  max_output_tokens: number;
  supports_vision: boolean;
  supports_function_calling: boolean;
  supports_json_mode: boolean;
  supports_streaming: boolean;
  supports_system_message: boolean;
  embedding_dimension: number;
}

/**
 * Model cost information (per 1K tokens in USD).
 */
export interface ModelCost {
  input_per_1k: number;
  output_per_1k: number;
  embedding_per_1k: number;
  image_per_unit: number;
}

/**
 * Model card with capabilities and cost.
 */
export interface ModelResponse {
  name: string;
  display_name: string;
  model_type: "llm" | "embedding" | "multimodal";
  provider: string;
  provider_display_name: string;
  description: string;
  deprecated: boolean;
  replacement?: string;
  capabilities: ModelCapabilities;
  cost?: ModelCost;
  tags: string[];
}

/**
 * Provider health status.
 */
export interface ProviderHealthResponse {
  available: boolean;
  latency_ms: number;
  error?: string;
  checked_at: string;
}

/**
 * Provider configuration with models.
 */
export interface ProviderResponse {
  name: string;
  display_name: string;
  provider_type: string;
  enabled: boolean;
  priority: number;
  description: string;
  models: ModelResponse[];
  health?: ProviderHealthResponse;
}

/**
 * Full models configuration response.
 */
export interface ModelsListResponse {
  providers: ProviderResponse[];
  default_llm_provider: string;
  default_llm_model: string;
  default_embedding_provider: string;
  default_embedding_model: string;
}

/**
 * LLM model item with provider info.
 */
export interface LlmModelItem {
  provider: string;
  provider_display_name: string;
  name: string;
  display_name: string;
  model_type: string;
  description: string;
  deprecated: boolean;
  replacement?: string;
  capabilities: ModelCapabilities;
  cost: ModelCost;
  tags: string[];
}

/**
 * LLM models response.
 */
export interface LlmModelsResponse {
  models: LlmModelItem[];
  default_provider: string;
  default_model: string;
}

/**
 * Embedding model item with provider info.
 */
export interface EmbeddingModelItem {
  provider: string;
  provider_display_name: string;
  dimension: number;
  name: string;
  display_name: string;
  model_type: string;
  description: string;
  deprecated: boolean;
  replacement?: string;
  capabilities: ModelCapabilities;
  cost: ModelCost;
  tags: string[];
}

/**
 * Embedding models response.
 */
export interface EmbeddingModelsResponse {
  models: EmbeddingModelItem[];
  default_provider: string;
  default_model: string;
}

// ============================================================================
// API Functions
// ============================================================================

/**
 * Fetch all providers and models configuration.
 *
 * @returns All configured providers with their models
 */
export async function fetchModelsConfig(): Promise<ModelsListResponse> {
  return apiClient<ModelsListResponse>("/models");
}

/**
 * Fetch LLM models only.
 *
 * @returns All LLM and multimodal models
 */
export async function fetchLlmModels(): Promise<LlmModelsResponse> {
  return apiClient<LlmModelsResponse>("/models/llm");
}

/**
 * Fetch embedding models only.
 *
 * @returns All embedding models
 */
export async function fetchEmbeddingModels(): Promise<EmbeddingModelsResponse> {
  return apiClient<EmbeddingModelsResponse>("/models/embedding");
}

/**
 * Fetch a specific provider by name.
 *
 * @param providerName - Provider identifier (e.g., "openai", "ollama")
 * @returns Provider details with all models
 */
export async function fetchProvider(
  providerName: string
): Promise<ProviderResponse> {
  return apiClient<ProviderResponse>(`/models/${providerName}`);
}

/**
 * Fetch a specific model by provider and model name.
 *
 * @param providerName - Provider identifier
 * @param modelName - Model identifier (e.g., "gpt-4o")
 * @returns Model card with capabilities and cost
 */
export async function fetchModel(
  providerName: string,
  modelName: string
): Promise<ModelResponse> {
  return apiClient<ModelResponse>(`/models/${providerName}/${modelName}`);
}

/**
 * Check health status of all enabled providers.
 *
 * @returns All providers with their health status
 */
export async function fetchProvidersHealth(): Promise<ProviderResponse[]> {
  return apiClient<ProviderResponse[]>("/models/health");
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Format model cost for display.
 *
 * @param cost - Cost in USD per 1K tokens
 * @returns Formatted cost string
 */
export function formatCost(cost: number): string {
  if (cost === 0) return "Free";
  if (cost < 0.001) return `$${cost.toFixed(6)}/1K`;
  if (cost < 0.01) return `$${cost.toFixed(4)}/1K`;
  return `$${cost.toFixed(3)}/1K`;
}

/**
 * Format context length for display.
 *
 * @param tokens - Context length in tokens
 * @returns Formatted string (e.g., "128K")
 */
export function formatContextLength(tokens: number): string {
  if (tokens >= 1000000) return `${(tokens / 1000000).toFixed(1)}M`;
  if (tokens >= 1000) return `${(tokens / 1000).toFixed(0)}K`;
  return tokens.toString();
}

/**
 * Get capability badges for a model.
 *
 * @param capabilities - Model capabilities
 * @returns Array of capability badge labels
 */
export function getCapabilityBadges(capabilities: ModelCapabilities): string[] {
  const badges: string[] = [];
  if (capabilities.supports_vision) badges.push("Vision");
  if (capabilities.supports_function_calling) badges.push("Functions");
  if (capabilities.supports_json_mode) badges.push("JSON");
  if (capabilities.supports_streaming) badges.push("Streaming");
  return badges;
}

/**
 * Check if a model is free to use.
 *
 * @param cost - Model cost information (optional)
 * @returns True if no cost or all costs are zero
 */
export function isModelFree(cost?: ModelCost): boolean {
  if (!cost) return true;
  return (
    (cost.input_per_1k ?? 0) === 0 &&
    (cost.output_per_1k ?? 0) === 0 &&
    (cost.embedding_per_1k ?? 0) === 0
  );
}
