"use client";

/**
 * React Query hooks for folder management.
 */

import {
  createFolder,
  deleteFolder,
  listFolders,
  updateFolder,
} from "@/lib/api/folders";
import { folderKeys } from "@/lib/api/query-keys";
import type {
  ConversationFolder,
  CreateFolderRequest,
  UpdateFolderRequest,
} from "@/types";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";

// ============================================================================
// List Folders
// ============================================================================

export function useFolders() {
  return useQuery({
    queryKey: folderKeys.list(),
    queryFn: listFolders,
    staleTime: 60_000, // 1 minute
  });
}

// ============================================================================
// Create Folder
// ============================================================================

export function useCreateFolder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateFolderRequest) => createFolder(data),
    onSuccess: (newFolder) => {
      // Optimistically update the cache
      queryClient.setQueryData<ConversationFolder[]>(folderKeys.list(), (old) =>
        old ? [...old, newFolder] : [newFolder]
      );
      toast.success("Folder created");
    },
    onError: () => {
      toast.error("Failed to create folder");
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.list() });
    },
  });
}

// ============================================================================
// Update Folder
// ============================================================================

export function useUpdateFolder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateFolderRequest }) =>
      updateFolder(id, data),
    onMutate: async ({ id, data }) => {
      await queryClient.cancelQueries({ queryKey: folderKeys.list() });

      const previousFolders = queryClient.getQueryData<ConversationFolder[]>(
        folderKeys.list()
      );

      if (previousFolders) {
        queryClient.setQueryData<ConversationFolder[]>(
          folderKeys.list(),
          previousFolders.map((folder) =>
            folder.id === id ? { ...folder, ...data } : folder
          )
        );
      }

      return { previousFolders };
    },
    onError: (_err, _, context) => {
      if (context?.previousFolders) {
        queryClient.setQueryData(folderKeys.list(), context.previousFolders);
      }
      toast.error("Failed to update folder");
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.list() });
    },
  });
}

// ============================================================================
// Delete Folder
// ============================================================================

export function useDeleteFolder() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => deleteFolder(id),
    onMutate: async (id) => {
      await queryClient.cancelQueries({ queryKey: folderKeys.list() });

      const previousFolders = queryClient.getQueryData<ConversationFolder[]>(
        folderKeys.list()
      );

      if (previousFolders) {
        queryClient.setQueryData<ConversationFolder[]>(
          folderKeys.list(),
          previousFolders.filter((folder) => folder.id !== id)
        );
      }

      return { previousFolders };
    },
    onError: (_err, _, context) => {
      if (context?.previousFolders) {
        queryClient.setQueryData(folderKeys.list(), context.previousFolders);
      }
      toast.error("Failed to delete folder");
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: folderKeys.list() });
    },
  });
}
