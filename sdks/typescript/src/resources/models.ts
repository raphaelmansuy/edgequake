/**
 * Models resource — model configuration and health.
 *
 * @module resources/models
 * @see edgequake/crates/edgequake-api/src/handlers/models.rs
 */

import type { ModelInfo, ProvidersHealth } from "../types/health.js";
import { Resource } from "./base.js";

export class ModelsResource extends Resource {
  /** List all available models. */
  async list(): Promise<ModelInfo[]> {
    return this._get("/api/v1/models");
  }

  /** List LLM models. */
  async listLlm(): Promise<ModelInfo[]> {
    return this._get("/api/v1/models/llm");
  }

  /** List embedding models. */
  async listEmbedding(): Promise<ModelInfo[]> {
    return this._get("/api/v1/models/embedding");
  }

  /** Check all provider health. */
  async health(): Promise<ProvidersHealth> {
    return this._get("/api/v1/models/health");
  }

  /** Get a specific provider's configuration. */
  async getProvider(provider: string): Promise<ModelInfo[]> {
    return this._get(`/api/v1/models/${provider}`);
  }

  /** Get a specific model's information. */
  async getModel(provider: string, model: string): Promise<ModelInfo> {
    return this._get(`/api/v1/models/${provider}/${model}`);
  }
}
