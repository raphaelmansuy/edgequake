"use client";

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
    DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import {
    useConversationList,
    useConversationStore,
    type Conversation,
} from "@/stores/use-conversation-store";
import { useTenantStore } from "@/stores/use-tenant-store";
import {
    ChevronLeft,
    ChevronRight,
    Edit2,
    MessageSquare,
    MoreVertical,
    Plus,
    Search,
    Trash2,
} from "lucide-react";
import { memo, useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";

// ============================================================================
// Conversation Item Component
// ============================================================================

interface ConversationItemProps {
  conversation: Conversation;
  isActive: boolean;
  onSelect: () => void;
  onRename: (title: string) => void;
  onDelete: () => void;
}

const ConversationItem = memo(function ConversationItem({
  conversation,
  isActive,
  onSelect,
  onRename,
  onDelete,
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
    const date = new Date(conversation.updatedAt);
    const now = new Date();
    const diffDays = Math.floor((now.getTime() - date.getTime()) / (1000 * 60 * 60 * 24));

    if (diffDays === 0) {
      return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    } else if (diffDays === 1) {
      return t("common.yesterday", "Yesterday");
    } else if (diffDays < 7) {
      return date.toLocaleDateString([], { weekday: "short" });
    } else {
      return date.toLocaleDateString([], { month: "short", day: "numeric" });
    }
  }, [conversation.updatedAt, t]);

  return (
    <div
      className={cn(
        "group relative flex items-center gap-2.5 px-3 py-2.5 rounded-lg cursor-pointer transition-all duration-150",
        isActive 
          ? "bg-primary/10 border border-primary/20 shadow-sm" 
          : "hover:bg-muted/70 border border-transparent"
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
      aria-pressed={isActive}
    >
      <div className={cn(
        "w-8 h-8 rounded-full flex items-center justify-center shrink-0 transition-colors",
        isActive ? "bg-primary/15" : "bg-muted/50"
      )}>
        <MessageSquare className={cn(
          "h-4 w-4",
          isActive ? "text-primary" : "text-muted-foreground"
        )} />
      </div>

      <div className="flex-1 min-w-0">
        {isEditing ? (
          <Input
            value={editTitle}
            onChange={(e) => setEditTitle(e.target.value)}
            onBlur={handleSaveTitle}
            onKeyDown={handleKeyDown}
            className="h-6 text-sm py-0 px-1"
            autoFocus
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <>
            <p className="text-sm font-medium truncate">{conversation.title}</p>
            <p className="text-xs text-muted-foreground">
              {conversation.messages.length} {t("query.messages", "messages")} · {formattedDate}
            </p>
          </>
        )}
      </div>

      {!isEditing && (
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
// Main Conversation History Panel
// ============================================================================

interface ConversationHistoryPanelProps {
  className?: string;
}

export function ConversationHistoryPanel({ className }: ConversationHistoryPanelProps) {
  const { t } = useTranslation();
  const [searchQuery, setSearchQuery] = useState("");
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [conversationToDelete, setConversationToDelete] = useState<string | null>(null);

  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  const {
    activeConversationId,
    historyPanelOpen,
    createConversation,
    setActiveConversation,
    renameConversation,
    deleteConversation,
    toggleHistoryPanel,
  } = useConversationStore();

  const conversations = useConversationList();

  // Filter conversations by search query and current tenant/workspace
  const filteredConversations = useMemo(() => {
    let filtered = conversations;

    // Filter by tenant/workspace if set
    if (selectedTenantId) {
      filtered = filtered.filter(
        (c) => !c.tenantId || c.tenantId === selectedTenantId
      );
    }
    if (selectedWorkspaceId) {
      filtered = filtered.filter(
        (c) => !c.workspaceId || c.workspaceId === selectedWorkspaceId
      );
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(
        (c) =>
          c.title.toLowerCase().includes(query) ||
          c.messages.some((m) => m.content.toLowerCase().includes(query))
      );
    }

    return filtered;
  }, [conversations, searchQuery, selectedTenantId, selectedWorkspaceId]);

  const handleNewConversation = useCallback(() => {
    createConversation(selectedTenantId ?? undefined, selectedWorkspaceId ?? undefined);
  }, [createConversation, selectedTenantId, selectedWorkspaceId]);

  const handleDeleteConversation = useCallback(() => {
    if (conversationToDelete) {
      deleteConversation(conversationToDelete);
      setConversationToDelete(null);
      setDeleteDialogOpen(false);
    }
  }, [conversationToDelete, deleteConversation]);

  // Collapsed state - just show toggle button
  if (!historyPanelOpen) {
    return (
      <div
        className={cn(
          "hidden md:flex flex-col items-center justify-start py-4 w-12 border-l bg-card/50 shrink-0 transition-all duration-200",
          className
        )}
      >
        <Button
          variant="ghost"
          size="icon"
          className="h-8 w-8 hover:bg-muted"
          onClick={toggleHistoryPanel}
          aria-label={t("query.history.expand", "Expand history")}
        >
          <ChevronLeft className="h-4 w-4" />
        </Button>
        <div className="mt-4 flex flex-col items-center gap-2">
          <MessageSquare className="h-4 w-4 text-muted-foreground" />
          <span
            className="text-xs text-muted-foreground"
            style={{ writingMode: "vertical-rl", textOrientation: "mixed" }}
          >
            {t("query.history.title", "History")}
          </span>
        </div>
      </div>
    );
  }

  return (
    <aside
      className={cn(
        "hidden md:flex flex-col w-72 border-l bg-card/50 backdrop-blur-sm shrink-0 transition-all duration-200",
        className
      )}
      aria-label={t("query.history.title", "Conversation history")}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b bg-card/80 shrink-0">
        <h2 className="text-sm font-semibold">{t("query.history.title", "History")}</h2>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 hover:bg-muted"
            onClick={handleNewConversation}
            aria-label={t("query.history.newConversation", "New conversation")}
          >
            <Plus className="h-4 w-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7 hover:bg-muted"
            onClick={toggleHistoryPanel}
            aria-label={t("query.history.collapse", "Collapse history")}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Search */}
      <div className="p-3 border-b shrink-0">
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
          <Input
            placeholder={t("query.history.search", "Search conversations...")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="h-9 pl-8 text-sm bg-muted/30 border-muted focus:bg-background transition-colors"
          />
        </div>
      </div>

      {/* Conversation List */}
      <ScrollArea className="flex-1">
        <div className="p-3 space-y-1">
          {filteredConversations.length === 0 ? (
            <div className="py-12 text-center">
              <div className="w-12 h-12 mx-auto rounded-full bg-muted/50 flex items-center justify-center mb-3">
                <MessageSquare className="h-6 w-6 text-muted-foreground/50" />
              </div>
              <p className="text-sm text-muted-foreground">
                {searchQuery
                  ? t("query.history.noResults", "No conversations found")
                  : t("query.history.empty", "No conversations yet")}
              </p>
              {!searchQuery && (
                <Button
                  variant="link"
                  size="sm"
                  onClick={handleNewConversation}
                  className="mt-2 text-primary"
                >
                  {t("query.history.startFirst", "Start your first conversation")}
                </Button>
              )}
            </div>
          ) : (
            filteredConversations.map((conversation) => (
              <ConversationItem
                key={conversation.id}
                conversation={conversation}
                isActive={conversation.id === activeConversationId}
                onSelect={() => setActiveConversation(conversation.id)}
                onRename={(title) => renameConversation(conversation.id, title)}
                onDelete={() => {
                  setConversationToDelete(conversation.id);
                  setDeleteDialogOpen(true);
                }}
              />
            ))
          )}
        </div>
      </ScrollArea>

      {/* Delete Confirmation Dialog */}
      <Dialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("query.history.deleteTitle", "Delete conversation?")}</DialogTitle>
            <DialogDescription>
              {t(
                "query.history.deleteDescription",
                "This action cannot be undone. This will permanently delete the conversation and all its messages."
              )}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteDialogOpen(false)}>
              {t("common.cancel", "Cancel")}
            </Button>
            <Button variant="destructive" onClick={handleDeleteConversation}>
              {t("common.delete", "Delete")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </aside>
  );
}

export default ConversationHistoryPanel;
