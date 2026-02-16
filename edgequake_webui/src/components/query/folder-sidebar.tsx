"use client";

/**
 * Folder Sidebar Component
 *
 * Displays and manages conversation folders with CRUD operations.
 * Supports drag-and-drop for moving conversations to folders.
 *
 * @implements FEAT0709 - Folder CRUD operations
 * @implements FEAT0710 - Move conversations to folders (drag-and-drop)
 * @enforces BR0707 - Folder names unique per user
 * @enforces BR0708 - Deleting folder moves conversations to root
 */

import { Button } from "@/components/ui/button";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
} from "@/components/ui/dialog";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import {
    useCreateFolder,
    useDeleteFolder,
    useFolders,
    useUpdateFolder,
} from "@/hooks/use-folders";
import {
    useMoveConversation,
    useMoveConversations,
} from "@/hooks/use-move-conversation";
import { cn } from "@/lib/utils";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import type { ConversationFolder } from "@/types";
import {
    Edit2,
    Folder,
    FolderOpen,
    FolderPlus,
    Inbox,
    Loader2,
    MoreVertical,
    Trash2,
} from "lucide-react";
import { memo, useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

/** Drag-and-drop data type for conversation IDs */
export const DND_CONVERSATION_TYPE = "application/x-conversation-ids";

// ============================================================================
// Folder Item Component
// ============================================================================

interface FolderItemProps {
  folder: ConversationFolder;
  isActive: boolean;
  onSelect: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
  /** Called when conversations are dropped onto this folder */
  onDrop?: (conversationIds: string[], folderId: string) => void;
}

const FolderItem = memo(function FolderItem({
  folder,
  isActive,
  onSelect,
  onRename,
  onDelete,
  onDrop,
}: FolderItemProps) {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(folder.name);
  const [isDragOver, setIsDragOver] = useState(false);

  const handleSaveName = useCallback(() => {
    if (editName.trim() && editName !== folder.name) {
      onRename(editName.trim());
    }
    setIsEditing(false);
  }, [editName, folder.name, onRename]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleSaveName();
      } else if (e.key === "Escape") {
        setEditName(folder.name);
        setIsEditing(false);
      }
    },
    [handleSaveName, folder.name]
  );

  // Drag-and-drop handlers for drop target
  const handleDragOver = useCallback((e: React.DragEvent) => {
    if (e.dataTransfer.types.includes(DND_CONVERSATION_TYPE)) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      setIsDragOver(true);
    }
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    if (!e.currentTarget.contains(e.relatedTarget as Node)) {
      setIsDragOver(false);
    }
  }, []);

  const handleDropEvent = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);
      const data = e.dataTransfer.getData(DND_CONVERSATION_TYPE);
      if (data && onDrop) {
        try {
          const ids: string[] = JSON.parse(data);
          if (ids.length > 0) {
            onDrop(ids, folder.id);
          }
        } catch {
          // Invalid data, ignore
        }
      }
    },
    [onDrop, folder.id],
  );

  return (
    <div
      className={cn(
        "group relative flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer transition-all duration-150",
        isDragOver
          ? "bg-primary/20 border-2 border-dashed border-primary ring-1 ring-primary/30"
          : isActive
            ? "bg-primary/10 text-primary"
            : "hover:bg-muted/60 text-muted-foreground hover:text-foreground",
      )}
      onClick={onSelect}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect();
        }
      }}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDropEvent}
    >
      {/* Icon - open when active or when dragging over */}
      {isDragOver || isActive ? (
        <FolderOpen className="h-3.5 w-3.5 shrink-0" />
      ) : (
        <Folder className="h-3.5 w-3.5 shrink-0" />
      )}

      {/* Name */}
      {isEditing ? (
        <Input
          value={editName}
          onChange={(e) => setEditName(e.target.value)}
          onBlur={handleSaveName}
          onKeyDown={handleKeyDown}
          className="h-5 text-xs py-0 px-1 flex-1"
          autoFocus
          onClick={(e) => e.stopPropagation()}
        />
      ) : (
        <span className="text-xs font-medium truncate flex-1">{folder.name}</span>
      )}

      {/* Actions */}
      {!isEditing && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-5 w-5 opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={(e) => e.stopPropagation()}
            >
              <MoreVertical className="h-3 w-3" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-32">
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                setIsEditing(true);
              }}
            >
              <Edit2 className="h-3 w-3 mr-2" />
              {t("common.rename", "Rename")}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onDelete();
              }}
              className="text-destructive focus:text-destructive"
            >
              <Trash2 className="h-3 w-3 mr-2" />
              {t("common.delete", "Delete")}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
    </div>
  );
});

// ============================================================================
// Loading Skeleton
// ============================================================================

function FolderSkeleton() {
  return (
    <div className="flex items-center gap-2 px-2 py-1.5">
      <Skeleton className="w-3.5 h-3.5 rounded" />
      <Skeleton className="h-3 flex-1" />
    </div>
  );
}

// ============================================================================
// Main Folder Sidebar Component
// ============================================================================

interface FolderSidebarProps {
  className?: string;
}

export function FolderSidebar({ className }: FolderSidebarProps) {
  const { t } = useTranslation();
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [folderToDelete, setFolderToDelete] = useState<string | null>(null);
  const [newFolderName, setNewFolderName] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [isRootDragOver, setIsRootDragOver] = useState(false);

  const store = useQueryUIStore();
  const { data: folders, isLoading } = useFolders();
  const createFolder = useCreateFolder();
  const updateFolder = useUpdateFolder();
  const deleteFolder = useDeleteFolder();
  const moveConversation = useMoveConversation();
  const moveConversations = useMoveConversations();

  /** Handle dropping conversations onto a folder (or root when folderId is null) */
  const handleDropOnFolder = useCallback(
    (conversationIds: string[], folderId: string | null) => {
      if (conversationIds.length === 1) {
        moveConversation.mutate({
          conversationId: conversationIds[0],
          folderId,
        });
      } else if (conversationIds.length > 1) {
        moveConversations.mutate({
          conversationIds,
          folderId,
        });
      }
    },
    [moveConversation, moveConversations],
  );

  // Root "All Conversations" drop target handlers
  const handleRootDragOver = useCallback((e: React.DragEvent) => {
    if (e.dataTransfer.types.includes(DND_CONVERSATION_TYPE)) {
      e.preventDefault();
      e.dataTransfer.dropEffect = "move";
      setIsRootDragOver(true);
    }
  }, []);

  const handleRootDragLeave = useCallback((e: React.DragEvent) => {
    if (!e.currentTarget.contains(e.relatedTarget as Node)) {
      setIsRootDragOver(false);
    }
  }, []);

  const handleRootDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsRootDragOver(false);
      const data = e.dataTransfer.getData(DND_CONVERSATION_TYPE);
      if (data) {
        try {
          const ids: string[] = JSON.parse(data);
          if (ids.length > 0) {
            handleDropOnFolder(ids, null);
          }
        } catch {
          // Invalid data, ignore
        }
      }
    },
    [handleDropOnFolder],
  );

  // Sort folders by position
  const sortedFolders = useMemo(() => {
    if (!folders) return [];
    return [...folders].sort((a, b) => a.position - b.position);
  }, [folders]);

  // Handle create new folder
  const handleCreateFolder = useCallback(async () => {
    if (!newFolderName.trim()) return;
    
    createFolder.mutate(
      { name: newFolderName.trim() },
      {
        onSuccess: () => {
          setNewFolderName("");
          setIsCreating(false);
        },
      }
    );
  }, [newFolderName, createFolder]);

  // Handle delete confirmation
  const handleDeleteConfirm = useCallback(() => {
    if (!folderToDelete) return;

    deleteFolder.mutate(folderToDelete, {
      onSuccess: () => {
        // If the deleted folder was selected, clear the filter
        if (store.filters.folderId === folderToDelete) {
          store.setFilters({ folderId: null });
        }
        setFolderToDelete(null);
        setDeleteDialogOpen(false);
      },
    });
  }, [folderToDelete, deleteFolder, store]);

  return (
    <div className={cn("space-y-1", className)}>
      {/* Unfiled - shows conversations without any folder, also a drop target */}
      <div
        className={cn(
          "flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer transition-all duration-150",
          isRootDragOver
            ? "bg-primary/20 border-2 border-dashed border-primary ring-1 ring-primary/30"
            : store.filters.unfiled && !store.filters.folderId
              ? "bg-primary/10 text-primary"
              : "hover:bg-muted/60 text-muted-foreground hover:text-foreground",
        )}
        onClick={() => store.setFilters({ folderId: null, unfiled: true })}
        role="button"
        tabIndex={0}
        onDragOver={handleRootDragOver}
        onDragLeave={handleRootDragLeave}
        onDrop={handleRootDrop}
      >
        <Inbox className="h-3.5 w-3.5 shrink-0" />
        <span className="text-xs font-medium">{t("query.folders.unfiled", "Unfiled")}</span>
      </div>

      {/* Folders List */}
      {isLoading ? (
        <div className="space-y-1">
          <FolderSkeleton />
          <FolderSkeleton />
          <FolderSkeleton />
        </div>
      ) : (
        <>
          {sortedFolders.map((folder) => (
            <FolderItem
              key={folder.id}
              folder={folder}
              isActive={store.filters.folderId === folder.id}
              onSelect={() => store.setFilters({ folderId: folder.id, unfiled: false })}
              onRename={(name) =>
                updateFolder.mutate({ id: folder.id, data: { name } })
              }
              onDelete={() => {
                setFolderToDelete(folder.id);
                setDeleteDialogOpen(true);
              }}
              onDrop={handleDropOnFolder}
            />
          ))}

          {/* Create New Folder */}
          {isCreating ? (
            <div className="flex items-center gap-2 px-2 py-1.5">
              <Folder className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
              <Input
                value={newFolderName}
                onChange={(e) => setNewFolderName(e.target.value)}
                onBlur={() => {
                  if (!newFolderName.trim()) setIsCreating(false);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleCreateFolder();
                  } else if (e.key === "Escape") {
                    setNewFolderName("");
                    setIsCreating(false);
                  }
                }}
                placeholder={t("query.folders.newPlaceholder", "Folder name")}
                className="h-5 text-xs py-0 px-1 flex-1"
                autoFocus
              />
              {createFolder.isPending && (
                <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
              )}
            </div>
          ) : (
            <Button
              variant="ghost"
              size="sm"
              className="w-full justify-start gap-2 h-7 px-2 text-xs text-muted-foreground hover:text-foreground"
              onClick={() => setIsCreating(true)}
            >
              <FolderPlus className="h-3.5 w-3.5" />
              {t("query.folders.new", "New Folder")}
            </Button>
          )}
        </>
      )}

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("query.folders.deleteTitle", "Delete folder?")}</DialogTitle>
            <DialogDescription>
              {t(
                "query.folders.deleteDescription",
                "This will remove the folder. Conversations in this folder will not be deleted."
              )}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDeleteDialogOpen(false)}
            >
              {t("common.cancel", "Cancel")}
            </Button>
            <Button
              variant="destructive"
              onClick={handleDeleteConfirm}
              disabled={deleteFolder.isPending}
            >
              {deleteFolder.isPending && (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              )}
              {t("common.delete", "Delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default FolderSidebar;
