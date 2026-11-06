/**
 * Connections resource — saved LLM provider endpoints.
 *
 * @module resources/connections
 * @see edgequake/crates/edgequake-api/src/handlers/connections.rs
 */

import type {
  Connection,
  ProbeResult,
  ProviderTestRequest,
  UpsertConnection,
} from "../types/connections.js";
import { Resource } from "./base.js";

export class ConnectionsResource extends Resource {
  /** GET /api/v1/connections */
  async list(): Promise<Connection[]> {
    return this._get("/api/v1/connections");
  }

  /** POST /api/v1/connections?scope=deployment */
  async create(body: UpsertConnection): Promise<Connection> {
    return this._post("/api/v1/connections?scope=deployment", body);
  }

  /** PUT /api/v1/connections/{id}?scope=deployment */
  async update(id: string, body: UpsertConnection): Promise<Connection> {
    return this._put(`/api/v1/connections/${id}?scope=deployment`, body);
  }

  /** DELETE /api/v1/connections/{id}?scope=deployment */
  async delete(id: string): Promise<void> {
    await this._del(`/api/v1/connections/${id}?scope=deployment`);
  }

  /** POST /api/v1/connections/{id}/test?scope=deployment */
  async test(id: string): Promise<ProbeResult> {
    return this._post(`/api/v1/connections/${id}/test?scope=deployment`);
  }

  /** POST /api/v1/providers/test — probe without storing a connection. */
  async testProvider(body: ProviderTestRequest): Promise<ProbeResult> {
    return this._post("/api/v1/providers/test", body);
  }
}
