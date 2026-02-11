/**
 * Provenance resource — entity provenance tracing.
 *
 * WHY: Updated to use proper EntityProvenanceResponse from lineage.ts.
 * Matches Rust EntityProvenanceResponse with sources, extraction_count,
 * and related_entities.
 *
 * @module resources/provenance
 * @see edgequake/crates/edgequake-api/src/handlers/provenance.rs
 */

import type { EntityProvenanceResponse } from "../types/lineage.js";
import { Resource } from "./base.js";

export class ProvenanceResource extends Resource {
  /** Get provenance information for an entity. */
  async get(entityId: string): Promise<EntityProvenanceResponse> {
    return this._get(`/api/v1/entities/${entityId}/provenance`);
  }
}
