/**
 * Lineage resource — entity and document lineage tracking.
 *
 * WHY: Updated to use proper lineage types from lineage.ts.
 * Returns EntityLineageResponse and DocumentGraphLineageResponse
 * which match Rust lineage_types.rs exactly.
 *
 * @module resources/lineage
 * @see edgequake/crates/edgequake-api/src/handlers/lineage.rs
 */

import type {
  DocumentGraphLineageResponse,
  EntityLineageResponse,
} from "../types/lineage.js";
import { Resource } from "./base.js";

export class LineageResource extends Resource {
  /** Get entity lineage — which documents contributed to an entity. */
  async entity(entityName: string): Promise<EntityLineageResponse> {
    return this._get(
      `/api/v1/lineage/entities/${encodeURIComponent(entityName)}`,
    );
  }

  /** Get document lineage — which entities were extracted from a document. */
  async document(documentId: string): Promise<DocumentGraphLineageResponse> {
    return this._get(`/api/v1/lineage/documents/${documentId}`);
  }
}
