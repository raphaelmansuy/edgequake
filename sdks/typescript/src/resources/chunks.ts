/**
 * Chunks resource — chunk-level detail access.
 *
 * WHY: Updated to use proper ChunkDetailResponse from lineage.ts.
 * Matches Rust ChunkDetailResponse with rich metadata (char_range,
 * entities, relationships, extraction_metadata).
 *
 * @module resources/chunks
 * @see edgequake/crates/edgequake-api/src/handlers/chunks.rs
 */

import type { ChunkDetailResponse, ChunkLineageResponse } from "../types/lineage.js";
import { Resource } from "./base.js";

export class ChunksResource extends Resource {
  /** Get chunk details by ID. */
  async get(chunkId: string): Promise<ChunkDetailResponse> {
    return this._get(`/api/v1/chunks/${chunkId}`);
  }

  /**
   * Get chunk lineage with parent document refs and position info.
   * @implements F3 — Every chunk contains parent_document_id and position info.
   * @implements F8 — PDF → Document → Chunk → Entity chain traceable.
   */
  async getLineage(chunkId: string): Promise<ChunkLineageResponse> {
    return this._get(`/api/v1/chunks/${chunkId}/lineage`);
  }
}
