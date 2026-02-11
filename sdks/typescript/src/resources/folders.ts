/**
 * Folders resource — conversation folder management.
 *
 * @module resources/folders
 * @see edgequake/crates/edgequake-api/src/handlers/conversations.rs
 */

import type {
  CreateFolderRequest,
  FolderInfo,
  UpdateFolderRequest,
} from "../types/conversations.js";
import { Resource } from "./base.js";

export class FoldersResource extends Resource {
  /** List all folders. */
  async list(): Promise<FolderInfo[]> {
    return this._get("/api/v1/folders");
  }

  /** Create a new folder. */
  async create(request: CreateFolderRequest): Promise<FolderInfo> {
    return this._post("/api/v1/folders", request);
  }

  /** Update a folder. */
  async update(
    folderId: string,
    request: UpdateFolderRequest,
  ): Promise<FolderInfo> {
    return this._patch(`/api/v1/folders/${folderId}`, request);
  }

  /** Delete a folder. */
  async delete(folderId: string): Promise<void> {
    await this._del(`/api/v1/folders/${folderId}`);
  }
}
