/**
 * Chunks resource — chunk-level detail access.
 *
 * @module resources/chunks
 * @see edgequake/crates/edgequake-api/src/handlers/chunks.rs
 */

import { Resource } from "./base.js";
import type { ChunkDetail } from "../types/health.js";

export class ChunksResource extends Resource {
  /** Get chunk details by ID. */
  async get(chunkId: string): Promise<ChunkDetail> {
    return this._get(`/api/v1/chunks/${chunkId}`);
  }
}
