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
  type: 'llm';
  status: ConnectionStatus;
  model: string;
  base_url?: string;
  config: Record<string, any>;
}

export interface EmbeddingProviderStatus {
  name: string;
  type: 'embedding';
  status: ConnectionStatus;
  model: string;
  dimension: number;
}

export interface StorageStatus {
  type: 'memory' | 'postgres';
  dimension: number;
  dimension_mismatch: boolean;
  namespace: string;
}

export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected' | 'error';

export interface StatusMetadata {
  checked_at: string; // ISO 8601
  uptime_seconds: number;
}
