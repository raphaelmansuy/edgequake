/**
 * Ollama resource — Ollama-compatible API endpoints.
 *
 * @module resources/ollama
 * @see edgequake/crates/edgequake-api/src/handlers/ollama.rs
 */

import type {
  OllamaProcess,
  OllamaTag,
  OllamaVersion,
} from "../types/health.js";
import { Resource } from "./base.js";

export class OllamaResource extends Resource {
  /** Get Ollama version. */
  async version(): Promise<OllamaVersion> {
    return this._get("/api/version");
  }

  /** List available model tags. */
  async tags(): Promise<OllamaTag[]> {
    return this._get("/api/tags");
  }

  /** List running model processes. */
  async ps(): Promise<OllamaProcess[]> {
    return this._get("/api/ps");
  }

  /** Generate text (Ollama-compatible). */
  async generate(request: {
    model: string;
    prompt: string;
    stream?: boolean;
  }): Promise<unknown> {
    return this._post("/api/generate", request);
  }

  /** Chat completion (Ollama-compatible). */
  async chat(request: {
    model: string;
    messages: Array<{ role: string; content: string }>;
    stream?: boolean;
  }): Promise<unknown> {
    return this._post("/api/chat", request);
  }
}
