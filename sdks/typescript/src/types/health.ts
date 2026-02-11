/**
 * Health check types.
 *
 * @module types/health
 * @see edgequake/crates/edgequake-api/src/handlers/health_types.rs
 */

export interface HealthResponse {
  status: string;
  version: string;
  storage_mode: string;
  workspace_id: string;
  components: ComponentHealth;
  llm_provider_name?: string;
  schema?: SchemaHealth;
  providers?: ProvidersHealth;
  pdf_storage_enabled?: boolean;
}

export interface ComponentHealth {
  kv_storage: boolean;
  vector_storage: boolean;
  graph_storage: boolean;
  llm_provider: boolean;
}

export interface SchemaHealth {
  latest_version?: number;
  migrations_applied: number;
  last_applied_at?: string;
}

export interface ProvidersHealth {
  llm?: LlmProviderHealth;
  embedding?: EmbeddingProviderHealth;
}

export interface LlmProviderHealth {
  provider: string;
  model: string;
  status: string;
}

export interface EmbeddingProviderHealth {
  provider: string;
  model: string;
  dimensions: number;
  status: string;
}

export interface ReadyResponse {
  ready: boolean;
}

export interface LiveResponse {
  alive: boolean;
}

// ── Settings / Models ─────────────────────────────────────────

export interface ProviderStatus {
  llm_provider: string;
  embedding_provider: string;
  available_providers: string[];
}

export interface AvailableProviders {
  providers: Array<{
    name: string;
    type: string;
    available: boolean;
    models?: string[];
  }>;
}

export interface ModelsResponse {
  models: ModelInfo[];
}

export interface ModelInfo {
  provider: string;
  name: string;
  type: "llm" | "embedding";
  context_window?: number;
  max_tokens?: number;
}

export interface LlmModelsResponse {
  models: ModelInfo[];
}

export interface EmbeddingModelsResponse {
  models: ModelInfo[];
}

export interface ProvidersHealthResponse {
  providers: Array<{
    name: string;
    healthy: boolean;
    latency_ms?: number;
    error?: string;
  }>;
}

export interface ProviderDetail {
  name: string;
  type: string;
  models: ModelInfo[];
  healthy: boolean;
}

export interface ModelDetail extends ModelInfo {
  healthy: boolean;
  latency_ms?: number;
}

// ── Lineage (moved to lineage.ts) ─────────────────────────────
// WHY: Re-export legacy aliases for backward compatibility.
// Actual types are now in lineage.ts with proper shapes matching Rust.
export type {
  ChunkDetail,
  DocumentLineage,
  EntityLineage,
  EntityProvenance,
} from "./lineage.js";

// ── Ollama Compatibility ──────────────────────────────────────

export interface OllamaVersion {
  version: string;
}

export interface OllamaTag {
  name: string;
  model: string;
  modified_at: string;
  size: number;
}

export interface OllamaTags {
  models: OllamaTag[];
}

export interface OllamaPs {
  models: OllamaProcess[];
}

export interface OllamaProcess {
  name: string;
  model: string;
  size: number;
}

export interface OllamaGenerateRequest {
  model: string;
  prompt: string;
  stream?: boolean;
}

export interface OllamaGenerateResponse {
  model: string;
  response: string;
  done: boolean;
}

export interface OllamaChatRequest {
  model: string;
  messages: Array<{ role: string; content: string }>;
  stream?: boolean;
}

export interface OllamaChatResponse {
  model: string;
  message: { role: string; content: string };
  done: boolean;
}

// ── WebSocket Events ──────────────────────────────────────────

export type WebSocketEvent =
  | { type: "progress"; track_id: string; step: string; progress: number }
  | { type: "complete"; track_id: string; result?: Record<string, unknown> }
  | { type: "error"; track_id: string; message: string };
