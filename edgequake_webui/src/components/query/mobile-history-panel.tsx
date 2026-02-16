"use client";

/**
 * Mobile History Panel Component
 *
 * A slide-over panel for accessing conversation history on mobile devices.
 * Uses the Sheet component for the sliding animation.
 */

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
    Sheet,
    SheetContent,
    SheetHeader,
    SheetTitle,
    SheetTrigger,
} from "@/components/ui/sheet";
import { Skeleton } from "@/components/ui/skeleton";
import {
    useConversations,
    useDeleteConversation,
} from "@/hooks/use-conversations";
import { cn } from "@/lib/utils";
import { useQueryUIStore } from "@/stores/use-query-ui-store";
import type { ServerConversation } from "@/types";
import {
    Archive,
    ChevronRight,
    Clock,
    Loader2,
    Menu,
    MessageSquare,
    Pin,
    Plus,
    Search,
    Trash2,
} from "lucide-react";
import { memo, useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderSidebar } from "./folder-sidebar";

// ============================================================================
// Conversation Item (Mobile-optimized)
// ============================================================================

interface MobileConversationItemProps {
  conversation: ServerConversation;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
}

const MobileConversationItem = memo(function MobileConversationItem({
  conversation,
  isActive,
  onSelect,
  onDelete,
}: MobileConversationItemProps) {
  const { t } = useTranslation();
  const [showDelete, setShowDelete] = useState(false);

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

  // Swipe to delete functionality
  const handleTouchStart = useCallback(() => {
    // Could implement swipe gestures here
  }, []);

  return (
    <div
      className={cn(
        "group flex items-center gap-3 px-4 py-3 border-b active:bg-muted transition-colors",
        isActive && "bg-primary/5 border-l-2 border-l-primary"
      )}
      onClick={onSelect}
      onTouchStart={handleTouchStart}
      role="button"
      tabIndex={0}
    >
      {/* Icon */}
      <div
        className={cn(
          "w-10 h-10 rounded-full flex items-center justify-center shrink-0",
          isActive ? "bg-primary/15" : "bg-muted/50"
        )}
      >
        <MessageSquare
          className={cn(
            "h-5 w-5",
            isActive ? "text-primary" : "text-muted-foreground"
          )}
        />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-1.5">
          <p className="text-sm font-medium truncate flex-1">
            {conversation.title}
          </p>
          {conversation.is_pinned && (
            <Pin className="h-3 w-3 text-amber-500 shrink-0" />
          )}
          {conversation.is_archived && (
            <Archive className="h-3 w-3 text-muted-foreground shrink-0" />
          )}
        </div>
        <div className="flex items-center gap-2 mt-0.5">
          <Clock className="h-3 w-3 text-muted-foreground" />
          <p className="text-xs text-muted-foreground">
            {formattedDate}
          </p>
          <span className="text-muted-foreground">·</span>
          <p className="text-xs text-muted-foreground">
            {conversation.message_count} {t("query.messages", "messages")}
          </p>
        </div>
        {conversation.last_message_preview && (
          <p className="text-xs text-muted-foreground truncate mt-1">
            {conversation.last_message_preview}
          </p>
        )}
      </div>

      {/* Delete button (swipe or long-press reveal) */}
      {showDelete && (
        <Button
          variant="destructive"
          size="icon"
          className="h-8 w-8 shrink-0"
          onClick={(e) => {
            e.stopPropagation();
            onDelete();
          }}
        >
          <Trash2 className="h-4 w-4" />
        </Button>
      )}

      {/* Chevron */}
      <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />
    </div>
  );
});

// ============================================================================
// Loading Skeleton
// ============================================================================

function MobileConversationSkeleton() {
  return (
    <div className="flex items-center gap-3 px-4 py-3 border-b">
      <Skeleton className="w-10 h-10 rounded-full" />
      <div className="flex-1 space-y-2">
        <Skeleton className="h-4 w-3/4" />
        <Skeleton className="h-3 w-1/2" />
      </div>
    </div>
  );
}

// ============================================================================
// Main Mobile History Panel
// ============================================================================

interface MobileHistoryPanelProps {
  className?: string;
}

export function MobileHistoryPanel({ className }: MobileHistoryPanelProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const store = useQueryUIStore();

  // Query with filters
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
  });

  const deleteConversation = useDeleteConversation();

  // Flatten pages
  const conversations = useMemo(() => {
    return data?.pages.flatMap((page) => page.items) ?? [];
  }, [data]);

  // Handle search
  const handleSearchChange = useCallback(
    (value: string) => {
      store.setFilters({ search: value });
    },
    [store]
  );

  // Handle conversation selection
  const handleSelectConversation = useCallback(
    (conversationId: string) => {
      store.setActiveConversation(conversationId);
      setOpen(false); // Close panel after selection on mobile
    },
    [store]
  );

  // Handle new conversation
  const handleNewConversation = useCallback(() => {
    store.setActiveConversation(null);
    setOpen(false);
  }, [store]);

  // Handle scroll to load more
  const handleScroll = useCallback(
    (e: React.UIEvent<HTMLDivElement>) => {
      const target = e.currentTarget;
      const scrolledToBottom =
        target.scrollHeight - target.scrollTop <= target.clientHeight + 100;

      if (scrolledToBottom && hasNextPage && !isFetchingNextPage) {
        fetchNextPage();
      }
    },
    [hasNextPage, isFetchingNextPage, fetchNextPage]
  );

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          className={cn("md:hidden h-9 w-9", className)}
          aria-label={t("query.history.open", "Open conversation history")}
        >
          <Menu className="h-5 w-5" />
        </Button>
      </SheetTrigger>
      <SheetContent side="left" className="w-full sm:w-96 p-0">
        {/* Header */}
        <SheetHeader className="px-4 py-3 border-b bg-muted/20">
          <div className="flex items-center justify-between">
            <SheetTitle className="text-base">
              {t("query.history.title", "Conversations")}
            </SheetTitle>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleNewConversation}
              aria-label={t("query.history.newConversation", "New conversation")}
            >
              <Plus className="h-4 w-4" />
            </Button>
          </div>
        </SheetHeader>

        {/* Search */}
        <div className="p-3 border-b">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder={t("query.history.search", "Search conversations...")}
              value={store.filters.search}
              onChange={(e) => handleSearchChange(e.target.value)}
              className="h-10 pl-10 text-sm"
            />
          </div>
        </div>

        {/* Folders */}
        <div className="p-3 border-b">
          <FolderSidebar />
        </div>

        {/* Conversation List */}
        <div
          className="flex-1 overflow-y-auto"
          onScroll={handleScroll}
          style={{ maxHeight: "calc(100vh - 220px)" }}
        >
          {isLoading ? (
            <div>
              {Array.from({ length: 5 }).map((_, i) => (
                <MobileConversationSkeleton key={i} />
              ))}
            </div>
          ) : conversations.length === 0 ? (
            <div className="py-16 text-center px-4">
              <div className="w-16 h-16 mx-auto rounded-full bg-muted/50 flex items-center justify-center mb-4">
                <MessageSquare className="h-8 w-8 text-muted-foreground/50" />
              </div>
              <p className="text-sm text-muted-foreground mb-2">
                {store.filters.search
                  ? t("query.history.noResults", "No conversations found")
                  : t("query.history.empty", "No conversations yet")}
              </p>
              {!store.filters.search && (
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleNewConversation}
                >
                  {t("query.history.startFirst", "Start your first conversation")}
                </Button>
              )}
            </div>
          ) : (
            <div>
              {conversations.map((conversation) => (
                <MobileConversationItem
                  key={conversation.id}
                  conversation={conversation}
                  isActive={conversation.id === store.activeConversationId}
                  onSelect={() => handleSelectConversation(conversation.id)}
                  onDelete={() => {
                    deleteConversation.mutate(conversation.id);
                    if (store.activeConversationId === conversation.id) {
                      store.setActiveConversation(null);
                    }
                  }}
                />
              ))}

              {/* Load more indicator */}
              {isFetchingNextPage && (
                <div className="flex items-center justify-center py-4">
                  <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
                </div>
              )}
            </div>
          )}
        </div>
      </SheetContent>
    </Sheet>
  );
}

export default MobileHistoryPanel;
