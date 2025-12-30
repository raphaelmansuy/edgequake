'use client';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useGraphStore, type GraphBookmark } from '@/stores/use-graph-store';
import { Bookmark, BookmarkPlus, MoreVertical, Pencil, Trash2, X } from 'lucide-react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

interface BookmarksPanelProps {
  className?: string;
  collapsed?: boolean;
}

export function BookmarksPanel({ className, collapsed = false }: BookmarksPanelProps) {
  const { t } = useTranslation();
  const [isCollapsed, setIsCollapsed] = useState(collapsed);
  const [newBookmarkName, setNewBookmarkName] = useState('');
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState('');

  const {
    bookmarks,
    saveBookmark,
    loadBookmark,
    deleteBookmark,
    renameBookmark,
  } = useGraphStore();

  const handleSaveBookmark = useCallback(() => {
    if (!newBookmarkName.trim()) {
      toast.error(t('graph.bookmarks.nameRequired', 'Please enter a bookmark name'));
      return;
    }
    
    const bookmark = saveBookmark(newBookmarkName.trim());
    if (bookmark) {
      toast.success(t('graph.bookmarks.saved', 'Bookmark saved'));
      setNewBookmarkName('');
    } else {
      toast.error(t('graph.bookmarks.saveFailed', 'Failed to save bookmark'));
    }
  }, [newBookmarkName, saveBookmark, t]);

  const handleLoadBookmark = useCallback((bookmark: GraphBookmark) => {
    loadBookmark(bookmark.id);
    toast.success(t('graph.bookmarks.loaded', 'Bookmark loaded: {{name}}', { name: bookmark.name }));
  }, [loadBookmark, t]);

  const handleDeleteBookmark = useCallback((bookmarkId: string) => {
    deleteBookmark(bookmarkId);
    toast.success(t('graph.bookmarks.deleted', 'Bookmark deleted'));
  }, [deleteBookmark, t]);

  const handleStartEdit = useCallback((bookmark: GraphBookmark) => {
    setEditingId(bookmark.id);
    setEditingName(bookmark.name);
  }, []);

  const handleSaveEdit = useCallback(() => {
    if (editingId && editingName.trim()) {
      renameBookmark(editingId, editingName.trim());
      setEditingId(null);
      setEditingName('');
      toast.success(t('graph.bookmarks.renamed', 'Bookmark renamed'));
    }
  }, [editingId, editingName, renameBookmark, t]);

  const handleCancelEdit = useCallback(() => {
    setEditingId(null);
    setEditingName('');
  }, []);

  if (isCollapsed) {
    return (
      <Button
        variant="outline"
        size="icon"
        className={`bg-background/80 backdrop-blur-sm relative ${className}`}
        onClick={() => setIsCollapsed(false)}
        aria-label={t('graph.bookmarks.show', 'Show bookmarks')}
        title={t('graph.bookmarks.show', 'Bookmarks')}
      >
        <Bookmark className="h-4 w-4" aria-hidden="true" />
        {bookmarks.length > 0 && (
          <span className="absolute -top-1 -right-1 h-4 w-4 rounded-full bg-primary text-[10px] font-medium text-primary-foreground flex items-center justify-center">
            {bookmarks.length}
          </span>
        )}
      </Button>
    );
  }

  return (
    <Card
      className={`bg-background/80 backdrop-blur-sm shadow-lg w-72 ${className}`}
    >
      <CardHeader className="p-3 pb-1 flex flex-row items-center justify-between">
        <CardTitle className="text-sm font-medium flex items-center gap-1.5">
          <Bookmark className="h-4 w-4" aria-hidden="true" />
          {t('graph.bookmarks.title', 'Bookmarks')}
        </CardTitle>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={() => setIsCollapsed(true)}
          aria-label={t('graph.bookmarks.collapse', 'Collapse bookmarks')}
        >
          <X className="h-3.5 w-3.5" />
        </Button>
      </CardHeader>

      <CardContent className="p-3 pt-1 space-y-3">
        {/* Save New Bookmark */}
        <div className="flex gap-2">
          <Input
            placeholder={t('graph.bookmarks.namePlaceholder', 'Bookmark name...')}
            value={newBookmarkName}
            onChange={(e) => setNewBookmarkName(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSaveBookmark()}
            className="h-8 text-xs"
          />
          <Button
            variant="outline"
            size="icon"
            className="h-8 w-8 shrink-0"
            onClick={handleSaveBookmark}
            aria-label={t('graph.bookmarks.save', 'Save bookmark')}
          >
            <BookmarkPlus className="h-4 w-4" />
          </Button>
        </div>

        {/* Bookmarks List */}
        {bookmarks.length === 0 ? (
          <p className="text-xs text-muted-foreground text-center py-4">
            {t('graph.bookmarks.empty', 'No bookmarks saved yet')}
          </p>
        ) : (
          <ScrollArea className="h-48">
            <div className="space-y-1">
              {bookmarks.map((bookmark) => (
                <div
                  key={bookmark.id}
                  className="flex items-center gap-2 p-2 rounded-md hover:bg-accent group"
                >
                  {editingId === bookmark.id ? (
                    <div className="flex gap-1 flex-1">
                      <Input
                        value={editingName}
                        onChange={(e) => setEditingName(e.target.value)}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter') handleSaveEdit();
                          if (e.key === 'Escape') handleCancelEdit();
                        }}
                        className="h-7 text-xs"
                        autoFocus
                      />
                      <Button size="icon" className="h-7 w-7" onClick={handleSaveEdit}>
                        ✓
                      </Button>
                      <Button size="icon" variant="ghost" className="h-7 w-7" onClick={handleCancelEdit}>
                        ✕
                      </Button>
                    </div>
                  ) : (
                    <>
                      <button
                        className="flex-1 text-left text-xs truncate hover:underline"
                        onClick={() => handleLoadBookmark(bookmark)}
                        title={bookmark.name}
                      >
                        {bookmark.name}
                      </button>
                      <span className="text-[10px] text-muted-foreground shrink-0">
                        {new Date(bookmark.createdAt).toLocaleDateString()}
                      </span>
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6 opacity-0 group-hover:opacity-100"
                          >
                            <MoreVertical className="h-3.5 w-3.5" />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onClick={() => handleStartEdit(bookmark)}>
                            <Pencil className="h-3.5 w-3.5 mr-2" />
                            {t('common.rename', 'Rename')}
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem
                            className="text-destructive"
                            onClick={() => handleDeleteBookmark(bookmark.id)}
                          >
                            <Trash2 className="h-3.5 w-3.5 mr-2" />
                            {t('common.delete', 'Delete')}
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </>
                  )}
                </div>
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
