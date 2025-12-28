"use client";

/**
 * Folder Sidebar Component
 *
 * Displays and manages conversation folders with CRUD operations.
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

// ============================================================================
// Folder Item Component
// ============================================================================

interface FolderItemProps {
  folder: ConversationFolder;
  isActive: boolean;
  onSelect: () => void;
  onRename: (name: string) => void;
  onDelete: () => void;
}

const FolderItem = memo(function FolderItem({
  folder,
  isActive,
  onSelect,
  onRename,
  onDelete,
}: FolderItemProps) {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [editName, setEditName] = useState(folder.name);

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

  return (
    <div
      className={cn(
        "group relative flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer transition-all duration-150",
        isActive
          ? "bg-primary/10 text-primary"
          : "hover:bg-muted/60 text-muted-foreground hover:text-foreground"
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
    >
      {/* Icon */}
      {isActive ? (
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

  const store = useQueryUIStore();
  const { data: folders, isLoading } = useFolders();
  const createFolder = useCreateFolder();
  const updateFolder = useUpdateFolder();
  const deleteFolder = useDeleteFolder();

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
      {/* All Conversations */}
      <div
        className={cn(
          "flex items-center gap-2 px-2 py-1.5 rounded-md cursor-pointer transition-all duration-150",
          !store.filters.folderId
            ? "bg-primary/10 text-primary"
            : "hover:bg-muted/60 text-muted-foreground hover:text-foreground"
        )}
        onClick={() => store.setFilters({ folderId: null })}
        role="button"
        tabIndex={0}
      >
        <Inbox className="h-3.5 w-3.5 shrink-0" />
        <span className="text-xs font-medium">{t("query.folders.all", "All Conversations")}</span>
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
              onSelect={() => store.setFilters({ folderId: folder.id })}
              onRename={(name) =>
                updateFolder.mutate({ id: folder.id, data: { name } })
              }
              onDelete={() => {
                setFolderToDelete(folder.id);
                setDeleteDialogOpen(true);
              }}
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
