/**
 * Lineage resource — entity and document lineage tracking.
 *
 * @module resources/lineage
 * @see edgequake/crates/edgequake-api/src/handlers/lineage.rs
 */

import type { DocumentLineage, EntityLineage } from "../types/health.js";
import { Resource } from "./base.js";

export class LineageResource extends Resource {
  /** Get entity lineage — which documents contributed to an entity. */
  async entity(entityName: string): Promise<EntityLineage> {
    return this._get(
      `/api/v1/lineage/entities/${encodeURIComponent(entityName)}`,
    );
  }

  /** Get document lineage — which entities were extracted from a document. */
  async document(documentId: string): Promise<DocumentLineage> {
    return this._get(`/api/v1/lineage/documents/${documentId}`);
  }
}
