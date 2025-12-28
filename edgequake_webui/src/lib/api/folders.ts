import type {
  ConversationFolder,
  CreateFolderRequest,
  UpdateFolderRequest,
} from "@/types";
import { api } from "./client";

/**
 * List all folders for the current user.
 */
export async function listFolders(): Promise<ConversationFolder[]> {
  // API returns array directly, not { items: [...] }
  const response = await api.get<ConversationFolder[]>("/folders");
  return response ?? [];
}

/**
 * Create a new folder.
 */
export async function createFolder(
  data: CreateFolderRequest
): Promise<ConversationFolder> {
  return api.post<ConversationFolder>("/folders", data);
}

/**
 * Update a folder.
 */
export async function updateFolder(
  folderId: string,
  data: UpdateFolderRequest
): Promise<ConversationFolder> {
  return api.patch<ConversationFolder>(`/folders/${folderId}`, data);
}

/**
 * Delete a folder.
 */
export async function deleteFolder(folderId: string): Promise<void> {
  return api.delete(`/folders/${folderId}`);
}
