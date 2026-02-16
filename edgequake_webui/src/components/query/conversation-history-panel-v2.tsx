"use client";

import { Badge } from "@/components/ui/badge";
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
    DropdownMenuSeparator,
    DropdownMenuSub,
    DropdownMenuSubContent,
    DropdownMenuSubTrigger,
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { ResizablePanel } from "@/components/ui/resizable-panel";
import { Skeleton } from "@/components/ui/skeleton";
import {
    useConversation,
    useConversations,
    useDeleteConversation,
    useDeleteConversations,
    useUpdateConversation,
} from "@/hooks/use-conversations";
import { useFolders } from "@/hooks/use-folders";
import { useMoveConversation, useMoveConversations } from "@/hooks/use-move-conversation";
import { cn } from "@/lib/utils";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import { useTenantStore } from "@/stores/use-tenant-store";
import type { ServerConversation } from "@/types";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
    Archive,
    ChevronLeft,
    ChevronRight,
    Download,
    Edit2,
    Folder,
    FolderInput,
    Inbox,
    Loader2,
    MessageSquare,
    MoreVertical,
    Pin,
    PinOff,
    Plus,
    Search,
    Share2,
    Trash2,
    X,
} from "lucide-react";
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useInView } from "react-intersection-observer";
import { ExportDialog } from "./export-dialog";
import { DND_CONVERSATION_TYPE, FolderSidebar } from "./folder-sidebar";
import { MigrationBanner } from "./migration-banner";
import { ShareDialog } from "./share-dialog";

// ============================================================================
// Conversation Item Component (Virtualized)
// ============================================================================

interface ConversationItemProps {
  conversation: ServerConversation;
  isActive: boolean;
  isSelected: boolean;
  isSelectionMode: boolean;
  /** All selected conversation IDs (for batch drag) */
  selectedIds: Set<string>;
  onSelect: () => void;
  onToggleSelection: () => void;
  onRename: (title: string) => void;
  onPin: () => void;
  onArchive: () => void;
  onExport: () => void;
  onShare: () => void;
  onDelete: () => void;
  onMoveToFolder: (folderId: string | null) => void;
  /** Available folders for "Move to Folder" submenu */
  folders: { id: string; name: string }[];
}

const ConversationItem = memo(function ConversationItem({
  conversation,
  isActive,
  isSelected,
  isSelectionMode,
  selectedIds,
  onSelect,
  onToggleSelection,
  onRename,
  onPin,
  onArchive,
  onExport,
  onShare,
  onDelete,
  onMoveToFolder,
  folders,
}: ConversationItemProps) {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [editTitle, setEditTitle] = useState(conversation.title);

  const handleSaveTitle = useCallback(() => {
    if (editTitle.trim() && editTitle !== conversation.title) {
      onRename(editTitle.trim());
    }
    setIsEditing(false);
  }, [editTitle, conversation.title, onRename]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleSaveTitle();
      } else if (e.key === "Escape") {
        setEditTitle(conversation.title);
        setIsEditing(false);
      }
    },
    [handleSaveTitle, conversation.title]
  );

  const formattedDate = useMemo(() => {
    const date = new Date(conversation.updated_at);
    const now = new Date();
    const diffDays = Math.floor(
      (now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24)
    );

    if (diffDays === 0) {
      return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    } else if (diffDays === 1) {
      return t("common.yesterday", "Yesterday");
    } else if (diffDays < 7) {
      return date.toLocaleDateString([], { weekday: "short" });
    } else {
      return date.toLocaleDateString([], { month: "short", day: "numeric" });
    }
  }, [conversation.updated_at, t]);

  // Handle click - if selection mode, toggle selection; otherwise select conversation
  const handleClick = useCallback(() => {
    if (isSelectionMode) {
      onToggleSelection();
    } else {
      onSelect();
    }
  }, [isSelectionMode, onToggleSelection, onSelect]);

  // Drag start: include all selected IDs if in selection mode, otherwise just this one
  const handleDragStart = useCallback(
    (e: React.DragEvent) => {
      const ids =
        isSelectionMode && isSelected && selectedIds.size > 0
          ? Array.from(selectedIds)
          : [conversation.id];
      e.dataTransfer.setData(DND_CONVERSATION_TYPE, JSON.stringify(ids));
      e.dataTransfer.effectAllowed = "move";
    },
    [conversation.id, isSelectionMode, isSelected, selectedIds],
  );

  return (
    <div
      className={cn(
        "group relative flex items-center gap-2 px-2.5 py-2 rounded-md cursor-pointer transition-all duration-150",
        isActive && !isSelectionMode
          ? "bg-primary/10 border border-primary/20"
          : isSelected
          ? "bg-accent border border-accent-foreground/20"
          : "hover:bg-muted/60 border border-transparent"
      )}
      onClick={handleClick}
      role="button"
      tabIndex={0}
      draggable
      onDragStart={handleDragStart}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          handleClick();
        }
      }}
      aria-pressed={isActive}
    >
      {/* Selection checkbox in selection mode */}
      {isSelectionMode && (
        <div
          className={cn(
            "w-4 h-4 rounded border-2 flex items-center justify-center shrink-0 transition-colors",
            isSelected
              ? "bg-primary border-primary"
              : "border-muted-foreground/40"
          )}
        >
          {isSelected && (
            <svg className="w-3 h-3 text-primary-foreground" viewBox="0 0 12 12">
              <path
                d="M10 3L4.5 8.5L2 6"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          )}
        </div>
      )}

      {/* Icon */}
      <div
        className={cn(
          "w-7 h-7 rounded-md flex items-center justify-center shrink-0 transition-colors",
          isActive ? "bg-primary/15" : "bg-muted/40"
        )}
      >
        <MessageSquare
          className={cn(
            "h-3.5 w-3.5",
            isActive ? "text-primary" : "text-muted-foreground"
          )}
        />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        {isEditing ? (
          <Input
            value={editTitle}
            onChange={(e) => setEditTitle(e.target.value)}
            onBlur={handleSaveTitle}
            onKeyDown={handleKeyDown}
            className="h-5 text-xs py-0 px-1"
            autoFocus
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <>
            <div className="flex items-center gap-1">
              <p className="text-xs font-medium truncate leading-tight flex-1">
                {conversation.title}
              </p>
              {conversation.is_pinned && (
                <Pin className="h-2.5 w-2.5 text-amber-500 shrink-0" />
              )}
            </div>
            <p className="text-[10px] text-muted-foreground leading-tight mt-0.5">
              {conversation.message_count}{" "}
              {t("query.messages", "messages")} · {formattedDate}
            </p>
          </>
        )}
      </div>

      {/* Actions dropdown (hidden in selection mode) */}
      {!isEditing && !isSelectionMode && (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
              onClick={(e) => e.stopPropagation()}
              aria-label={t("common.moreOptions", "More options")}
            >
              <MoreVertical className="h-3 w-3" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
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
                onPin();
              }}
            >
              {conversation.is_pinned ? (
                <>
                  <PinOff className="h-3 w-3 mr-2" />
                  {t("common.unpin", "Unpin")}
                </>
              ) : (
                <>
                  <Pin className="h-3 w-3 mr-2" />
                  {t("common.pin", "Pin")}
                </>
              )}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onArchive();
              }}
            >
              <Archive className="h-3 w-3 mr-2" />
              {conversation.is_archived
                ? t("common.unarchive", "Unarchive")
                : t("common.archive", "Archive")}
            </DropdownMenuItem>

            {/* Move to Folder Submenu */}
            <DropdownMenuSub>
              <DropdownMenuSubTrigger>
                <FolderInput className="h-3 w-3 mr-2" />
                {t("query.moveToFolder", "Move to Folder")}
              </DropdownMenuSubTrigger>
              <DropdownMenuSubContent className="w-40">
                {/* "Unfiled" (root / no folder) */}
                <DropdownMenuItem
                  onClick={(e) => {
                    e.stopPropagation();
                    onMoveToFolder(null);
                  }}
                  disabled={!conversation.folder_id}
                >
                  <Inbox className="h-3 w-3 mr-2" />
                  {t("query.folders.unfiled", "Unfiled")}
                </DropdownMenuItem>
                {folders.length > 0 && <DropdownMenuSeparator />}
                {folders.map((folder) => (
                  <DropdownMenuItem
                    key={folder.id}
                    onClick={(e) => {
                      e.stopPropagation();
                      onMoveToFolder(folder.id);
                    }}
                    disabled={conversation.folder_id === folder.id}
                  >
                    <Folder className="h-3 w-3 mr-2" />
                    <span className="truncate">{folder.name}</span>
                  </DropdownMenuItem>
                ))}
                {folders.length === 0 && (
                  <DropdownMenuItem disabled>
                    <span className="text-xs text-muted-foreground italic">
                      {t("query.folders.noFolders", "No folders yet")}
                    </span>
                  </DropdownMenuItem>
                )}
              </DropdownMenuSubContent>
            </DropdownMenuSub>

            <DropdownMenuSeparator />
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onExport();
              }}
            >
              <Download className="h-3 w-3 mr-2" />
              {t("common.export", "Export")}
            </DropdownMenuItem>
            <DropdownMenuItem
              onClick={(e) => {
                e.stopPropagation();
                onShare();
              }}
            >
              <Share2 className="h-3 w-3 mr-2" />
              {t("common.share", "Share")}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
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
// Filter Bar Component
// ============================================================================

interface FilterBarProps {
  onClose: () => void;
}

function FilterBar({ onClose }: FilterBarProps) {
  const { t } = useTranslation();
  const { filters, setFilters, resetFilters } = useQueryUIStore();

  const hasActiveFilters = useMemo(() => {
    return (
      filters.pinned !== null ||
      filters.archived ||
      (filters.mode && filters.mode.length > 0) ||
      filters.dateFrom ||
      filters.dateTo
    );
  }, [filters]);

  return (
    <div className="border-b bg-muted/20 p-2 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium">{t("query.filters", "Filters")}</span>
        <div className="flex items-center gap-1">
          {hasActiveFilters && (
            <Button
              variant="ghost"
              size="sm"
              className="h-5 px-1.5 text-xs"
              onClick={resetFilters}
            >
              {t("common.clear", "Clear")}
            </Button>
          )}
          <Button variant="ghost" size="icon" className="h-5 w-5" onClick={onClose}>
            <X className="h-3 w-3" />
          </Button>
        </div>
      </div>
      <div className="flex flex-wrap gap-1.5">
        <Badge
          variant={filters.pinned === true ? "default" : "outline"}
          className="cursor-pointer text-[10px] px-2 py-0.5"
          onClick={() =>
            setFilters({ pinned: filters.pinned === true ? null : true })
          }
        >
          <Pin className="h-2.5 w-2.5 mr-1" />
          {t("query.pinned", "Pinned")}
        </Badge>
        <Badge
          variant={filters.archived ? "default" : "outline"}
          className="cursor-pointer text-[10px] px-2 py-0.5"
          onClick={() => setFilters({ archived: !filters.archived })}
        >
          <Archive className="h-2.5 w-2.5 mr-1" />
          {t("query.archived", "Archived")}
        </Badge>
      </div>
    </div>
  );
}

// ============================================================================
// Selection Toolbar Component
// ============================================================================

interface SelectionToolbarProps {
  selectedCount: number;
  onDelete: () => void;
  onClear: () => void;
  onMoveToFolder: (folderId: string | null) => void;
  folders: { id: string; name: string }[];
}

function SelectionToolbar({ selectedCount, onDelete, onClear, onMoveToFolder, folders }: SelectionToolbarProps) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center justify-between px-3 py-2 bg-accent border-b">
      <span className="text-xs font-medium">
        {selectedCount} {t("common.selected", "selected")}
      </span>
      <div className="flex items-center gap-1">
        {/* Move to Folder */}
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-xs"
            >
              <FolderInput className="h-3 w-3 mr-1" />
              {t("query.moveToFolder", "Move")}
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end" className="w-40">
            <DropdownMenuItem onClick={() => onMoveToFolder(null)}>
              <Inbox className="h-3 w-3 mr-2" />
              {t("query.folders.unfiled", "Unfiled")}
            </DropdownMenuItem>
            {folders.length > 0 && <DropdownMenuSeparator />}
            {folders.map((folder) => (
              <DropdownMenuItem
                key={folder.id}
                onClick={() => onMoveToFolder(folder.id)}
              >
                <Folder className="h-3 w-3 mr-2" />
                <span className="truncate">{folder.name}</span>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
        <Button
          variant="ghost"
          size="sm"
          className="h-6 px-2 text-xs text-destructive hover:text-destructive"
          onClick={onDelete}
        >
          <Trash2 className="h-3 w-3 mr-1" />
          {t("common.delete", "Delete")}
        </Button>
        <Button variant="ghost" size="sm" className="h-6 px-2 text-xs" onClick={onClear}>
          <X className="h-3 w-3 mr-1" />
          {t("common.cancel", "Cancel")}
        </Button>
      </div>
    </div>
  );
}

// ============================================================================
// Loading Skeleton
// ============================================================================

function ConversationSkeleton() {
  return (
    <div className="flex items-center gap-2 px-2.5 py-2">
      <Skeleton className="w-7 h-7 rounded-md" />
      <div className="flex-1 space-y-1">
        <Skeleton className="h-3 w-3/4" />
        <Skeleton className="h-2 w-1/2" />
      </div>
    </div>
  );
}

// ============================================================================
// Main Conversation History Panel (V2 - Server-synced)
// ============================================================================

interface ConversationHistoryPanelV2Props {
  className?: string;
}

export function ConversationHistoryPanelV2({ className }: ConversationHistoryPanelV2Props) {
  const { t } = useTranslation();
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [conversationToDelete, setConversationToDelete] = useState<string | null>(null);
  const [showFilters, setShowFilters] = useState(false);
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [shareDialogOpen, setShareDialogOpen] = useState(false);
  const [selectedConversationForAction, setSelectedConversationForAction] = useState<string | null>(null);
  const parentRef = useRef<HTMLDivElement>(null);

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const _tenantStore = useTenantStore(); // Available for future multi-tenant filtering
  const store = useQueryUIStore();

  // Infinite scroll trigger
  const { ref: loadMoreRef, inView } = useInView({ threshold: 0.1 });

  // Query with filters from store
  const {
    data,
    isLoading,
    isFetchingNextPage,
    hasNextPage,
    fetchNextPage,
  } = useConversations({
    mode: store.filters.mode ?? undefined,
    archived: store.filters.archived,
    pinned: store.filters.pinned ?? undefined,
    folder_id: store.filters.folderId ?? undefined,
    unfiled: store.filters.unfiled || undefined,
    search: store.filters.search || undefined,
    date_from: store.filters.dateFrom ?? undefined,
    date_to: store.filters.dateTo ?? undefined,
    sort: store.sort.field,
    order: store.sort.order,
  });

  // Mutations
  const updateConversation = useUpdateConversation();
  const deleteConversation = useDeleteConversation();
  const deleteConversationsBatch = useDeleteConversations();
  const moveConversation = useMoveConversation();
  const moveConversations = useMoveConversations();

  // Folder data for "Move to Folder" submenu
  const { data: folders } = useFolders();
  const folderList = useMemo(() => {
    if (!folders) return [];
    return folders
      .sort((a, b) => a.position - b.position)
      .map((f) => ({ id: f.id, name: f.name }));
  }, [folders]);

  // Fetch full conversation data for export/share
  const { data: conversationForAction } = useConversation(selectedConversationForAction);

  // Flatten pages
  const conversations = useMemo(() => {
    return data?.pages.flatMap((page) => page.items) ?? [];
  }, [data]);

  // Virtualizer for large lists
  const virtualizer = useVirtualizer({
    count: conversations.length + (hasNextPage ? 1 : 0),
    getScrollElement: () => parentRef.current,
    estimateSize: () => 52, // Estimated height of each item
    overscan: 5,
  });

  // Load more when trigger is in view
  useEffect(() => {
    if (inView && hasNextPage && !isFetchingNextPage) {
      fetchNextPage();
    }
  }, [inView, hasNextPage, isFetchingNextPage, fetchNextPage]);

  // Debounced search
  const handleSearchChange = useCallback(
    (value: string) => {
      store.setFilters({ search: value });
    },
    [store]
  );

  // Handle new conversation (TODO: integrate with create mutation)
  const handleNewConversation = useCallback(() => {
    // Will be handled by QueryInterface - just clear active conversation
    store.setActiveConversation(null);
  }, [store]);

  // Handle delete confirmation
  const handleDeleteConfirm = useCallback(() => {
    if (store.isSelectionMode && store.selectedIds.size > 0) {
      deleteConversationsBatch.mutate(Array.from(store.selectedIds));
      store.clearSelection();
      store.setSelectionMode(false);
    } else if (conversationToDelete) {
      deleteConversation.mutate(conversationToDelete);
      if (store.activeConversationId === conversationToDelete) {
        store.setActiveConversation(null);
      }
      setConversationToDelete(null);
    }
    setDeleteDialogOpen(false);
  }, [
    store,
    conversationToDelete,
    deleteConversation,
    deleteConversationsBatch,
  ]);

  // Handle batch delete trigger
  const handleBatchDelete = useCallback(() => {
    setDeleteDialogOpen(true);
  }, []);

  // Collapsed state - just show toggle button
  if (!store.historyPanelOpen) {
    return (
      <div
        className={cn(
          "hidden md:flex flex-col items-center justify-start py-3 w-10 border-l bg-card/50 shrink-0 transition-all duration-200",
          className
        )}
      >
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 hover:bg-muted"
          onClick={store.toggleHistoryPanel}
          aria-label={t("query.history.expand", "Expand history")}
        >
          <ChevronLeft className="h-3.5 w-3.5" />
        </Button>
        <div className="mt-3 flex flex-col items-center gap-1.5">
          <MessageSquare className="h-3.5 w-3.5 text-muted-foreground" />
          <span
            className="text-[10px] text-muted-foreground font-medium"
            style={{ writingMode: "vertical-rl", textOrientation: "mixed" }}
          >
            {t("query.history.title", "History")}
          </span>
        </div>
      </div>
    );
  }

  return (
    <ResizablePanel
      side="right"
      defaultWidth={280}
      minWidth={240}
      maxWidth={500}
      storageKey="conversation-history-panel-width"
      ariaLabel="Resize history panel"
      className="hidden md:flex"
    >
      <aside
        className={cn(
          "flex flex-col w-full h-full min-h-0 border-l bg-card/50 backdrop-blur-sm transition-all duration-200",
          className
        )}
        aria-label={t("query.history.title", "Conversation history")}
      >
      {/* Migration Banner */}
      <MigrationBanner />

      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b bg-muted/20 shrink-0">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
          {t("query.history.title", "History")}
        </h2>
        <div className="flex items-center gap-0.5">
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 hover:bg-muted"
            onClick={handleNewConversation}
            aria-label={t("query.history.newConversation", "New conversation")}
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-6 w-6 hover:bg-muted"
            onClick={store.toggleHistoryPanel}
            aria-label={t("query.history.collapse", "Collapse history")}
          >
            <ChevronRight className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* Selection Toolbar */}
      {store.isSelectionMode && store.selectedIds.size > 0 && (
        <SelectionToolbar
          selectedCount={store.selectedIds.size}
          onDelete={handleBatchDelete}
          onClear={() => {
            store.clearSelection();
            store.setSelectionMode(false);
          }}
          onMoveToFolder={(folderId) =>
            moveConversations.mutate({
              conversationIds: Array.from(store.selectedIds),
              folderId,
            })
          }
          folders={folderList}
        />
      )}

      {/* Search */}
      <div className="p-2 border-b shrink-0">
        <div className="relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
          <Input
            placeholder={t("query.history.search", "Search conversations...")}
            value={store.filters.search}
            onChange={(e) => handleSearchChange(e.target.value)}
            className="h-7 pl-7 text-xs bg-muted/30 border-muted focus:bg-background transition-colors"
          />
        </div>
      </div>

      {/* Folders Section */}
      <div className="p-2 border-b shrink-0">
        <FolderSidebar />
      </div>

      {/* Filter Bar (collapsible) */}
      {showFilters && <FilterBar onClose={() => setShowFilters(false)} />}

      {/* Conversation List (Virtualized) */}
      <div
        ref={parentRef}
        className="flex-1 overflow-auto"
        style={{ contain: "strict" }}
      >
        {isLoading ? (
          <div className="p-2 space-y-1">
            {Array.from({ length: 5 }).map((_, i) => (
              <ConversationSkeleton key={i} />
            ))}
          </div>
        ) : conversations.length === 0 ? (
          <div className="py-10 text-center">
            <div className="w-10 h-10 mx-auto rounded-full bg-muted/50 flex items-center justify-center mb-2">
              <MessageSquare className="h-5 w-5 text-muted-foreground/50" />
            </div>
            <p className="text-xs text-muted-foreground">
              {store.filters.search
                ? t("query.history.noResults", "No conversations found")
                : t("query.history.empty", "No conversations yet")}
            </p>
            {!store.filters.search && (
              <Button
                variant="link"
                size="sm"
                onClick={handleNewConversation}
                className="mt-1 text-xs text-primary"
              >
                {t("query.history.startFirst", "Start your first conversation")}
              </Button>
            )}
          </div>
        ) : (
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              width: "100%",
              position: "relative",
            }}
          >
            {virtualizer.getVirtualItems().map((virtualItem) => {
              const isLoader = virtualItem.index >= conversations.length;
              const conversation = conversations[virtualItem.index];

              if (isLoader) {
                return (
                  <div
                    key="loader"
                    ref={loadMoreRef}
                    style={{
                      position: "absolute",
                      top: 0,
                      left: 0,
                      width: "100%",
                      height: `${virtualItem.size}px`,
                      transform: `translateY(${virtualItem.start}px)`,
                    }}
                    className="flex items-center justify-center py-2"
                  >
                    {isFetchingNextPage && (
                      <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                    )}
                  </div>
                );
              }

              return (
                <div
                  key={conversation.id}
                  style={{
                    position: "absolute",
                    top: 0,
                    left: 0,
                    width: "100%",
                    height: `${virtualItem.size}px`,
                    transform: `translateY(${virtualItem.start}px)`,
                    padding: "0 0.5rem",
                  }}
                >
                  <ConversationItem
                    conversation={conversation}
                    isActive={conversation.id === store.activeConversationId}
                    isSelected={store.selectedIds.has(conversation.id)}
                    isSelectionMode={store.isSelectionMode}
                    selectedIds={store.selectedIds}
                    onSelect={() => store.setActiveConversation(conversation.id)}
                    onToggleSelection={() => store.toggleSelection(conversation.id)}
                    onRename={(title) =>
                      updateConversation.mutate({
                        id: conversation.id,
                        data: { title },
                      })
                    }
                    onPin={() =>
                      updateConversation.mutate({
                        id: conversation.id,
                        data: { is_pinned: !conversation.is_pinned },
                      })
                    }
                    onArchive={() =>
                      updateConversation.mutate({
                        id: conversation.id,
                        data: { is_archived: !conversation.is_archived },
                      })
                    }
                    onExport={() => {
                      setSelectedConversationForAction(conversation.id);
                      setExportDialogOpen(true);
                    }}
                    onShare={() => {
                      setSelectedConversationForAction(conversation.id);
                      setShareDialogOpen(true);
                    }}
                    onDelete={() => {
                      setConversationToDelete(conversation.id);
                      setDeleteDialogOpen(true);
                    }}
                    onMoveToFolder={(folderId) =>
                      moveConversation.mutate({
                        conversationId: conversation.id,
                        folderId,
                      })
                    }
                    folders={folderList}
                  />
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {store.isSelectionMode && store.selectedIds.size > 0
                ? t("query.history.deleteMultipleTitle", "Delete {{count}} conversations?", {
                    count: store.selectedIds.size,
                  })
                : t("query.history.deleteTitle", "Delete conversation?")}
            </DialogTitle>
            <DialogDescription>
              {t(
                "query.history.deleteDescription",
                "This action cannot be undone. This will permanently delete the conversation and all its messages."
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
              disabled={deleteConversation.isPending || deleteConversationsBatch.isPending}
            >
              {(deleteConversation.isPending || deleteConversationsBatch.isPending) && (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              )}
              {t("common.delete", "Delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Export Dialog */}
      <ExportDialog
        conversation={conversationForAction ?? null}
        open={exportDialogOpen}
        onOpenChange={(open) => {
          setExportDialogOpen(open);
          if (!open) setSelectedConversationForAction(null);
        }}
      />

      {/* Share Dialog */}
      <ShareDialog
        conversation={conversationForAction ?? null}
        open={shareDialogOpen}
        onOpenChange={(open) => {
          setShareDialogOpen(open);
          if (!open) setSelectedConversationForAction(null);
        }}
      />
    </aside>
    </ResizablePanel>
  );
}

export default ConversationHistoryPanelV2;
