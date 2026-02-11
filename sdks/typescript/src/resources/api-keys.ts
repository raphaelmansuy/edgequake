/**
 * API Keys resource — create, list, revoke API keys.
 *
 * @module resources/api-keys
 * @see edgequake/crates/edgequake-api/src/handlers/auth.rs
 */

import { Resource } from "./base.js";
import type {
  CreateApiKeyRequest,
  ApiKeyResponse,
  ApiKeyInfo,
} from "../types/auth.js";

export class ApiKeysResource extends Resource {
  /** Create a new API key. */
  async create(request: CreateApiKeyRequest): Promise<ApiKeyResponse> {
    return this._post("/api/v1/api-keys", request);
  }

  /** List all API keys. */
  async list(): Promise<ApiKeyInfo[]> {
    return this._get("/api/v1/api-keys");
  }

  /** Revoke (delete) an API key. */
  async revoke(keyId: string): Promise<void> {
    await this._del(`/api/v1/api-keys/${keyId}`);
  }
}
