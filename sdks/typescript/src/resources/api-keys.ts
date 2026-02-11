/**
 * API Keys resource — create, list, revoke API keys.
 *
 * WHY: Updated to match Rust ListApiKeysResponse and RevokeApiKeyResponse.
 * @module resources/api-keys
 * @see edgequake/crates/edgequake-api/src/handlers/auth.rs
 */

import type {
  ApiKeyResponse,
  CreateApiKeyRequest,
  ListApiKeysResponse,
  RevokeApiKeyResponse,
} from "../types/auth.js";
import { Resource } from "./base.js";

export class ApiKeysResource extends Resource {
  /** Create a new API key. */
  async create(request: CreateApiKeyRequest): Promise<ApiKeyResponse> {
    return this._post("/api/v1/api-keys", request);
  }

  /**
   * List all API keys.
   * WHY: Rust returns ListApiKeysResponse { keys, total, page, page_size, total_pages }.
   */
  async list(query?: {
    page?: number;
    page_size?: number;
  }): Promise<ListApiKeysResponse> {
    const params = new URLSearchParams();
    if (query?.page) params.set("page", String(query.page));
    if (query?.page_size) params.set("page_size", String(query.page_size));
    const qs = params.toString();
    return this._get(`/api/v1/api-keys${qs ? `?${qs}` : ""}`);
  }

  /**
   * Revoke (delete) an API key.
   * WHY: Rust returns RevokeApiKeyResponse { key_id, message }.
   */
  async revoke(keyId: string): Promise<RevokeApiKeyResponse> {
    return this._del(`/api/v1/api-keys/${keyId}`);
  }
}
