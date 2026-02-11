/**
 * Provenance resource — entity provenance tracing.
 *
 * @module resources/provenance
 * @see edgequake/crates/edgequake-api/src/handlers/provenance.rs
 */

import type { EntityProvenance } from "../types/health.js";
import { Resource } from "./base.js";

export class ProvenanceResource extends Resource {
  /** Get provenance information for an entity. */
  async get(entityId: string): Promise<EntityProvenance> {
    return this._get(`/api/v1/entities/${entityId}/provenance`);
  }
}
