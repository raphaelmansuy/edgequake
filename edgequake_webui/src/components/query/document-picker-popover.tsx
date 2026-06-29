'use client';
/**
 * @module DocumentPickerPopover
 * @description Popover for selecting specific documents to scope a query.
 * Features type-ahead search (debounced 300ms), checkbox selection, and
 * a selection summary. Designed to be maximally non-bloated.
 *
 * @implements SPEC-031: Document scope selection
 */

import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { useDocumentSearch } from '@/hooks/use-document-search';
import { cn } from '@/lib/utils';
import type { DocumentSearchItem } from '@/types';
import { FileText, Loader2, Plus, Search, X } from 'lucide-react';
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface DocumentPickerPopoverProps {
  /** Currently selected document IDs */
  selectedIds: string[];
  /** Callback when selection changes */
  onSelectionChange: (ids: string[]) => void;
  /** Whether the picker is disabled */
  disabled?: boolean;
  /** Custom trigger element (defaults to [+ Add] button) */
  trigger?: React.ReactNode;
}

export function DocumentPickerPopover({
  selectedIds,
  onSelectionChange,
  disabled = false,
  trigger,
}: DocumentPickerPopoverProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');

  const { data: results, isLoading } = useDocumentSearch(search, open);

  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  const toggle = useCallback(
    (item: DocumentSearchItem) => {
      if (selectedSet.has(item.id)) {
        onSelectionChange(selectedIds.filter((id) => id !== item.id));
      } else {
        onSelectionChange([...selectedIds, item.id]);
      }
    },
    [selectedIds, selectedSet, onSelectionChange],
  );

  const clearAll = useCallback(() => {
    onSelectionChange([]);
  }, [onSelectionChange]);

  // Sort: selected items first, then alphabetical
  const sortedResults = useMemo(() => {
    return [...results].sort((a, b) => {
      const aSelected = selectedSet.has(a.id) ? 0 : 1;
      const bSelected = selectedSet.has(b.id) ? 0 : 1;
      if (aSelected !== bSelected) return aSelected - bSelected;
      return a.title.localeCompare(b.title);
    });
  }, [results, selectedSet]);

  const handleOpenChange = (next: boolean) => {
    setOpen(next);
    if (!next) setSearch('');
  };

  return (
    <Popover open={open} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>
        {trigger ?? (
          <Button
            variant="ghost"
            size="sm"
            disabled={disabled}
            className="gap-1.5 h-7 text-xs"
            aria-label={t('query.scope.addDocuments', 'Add documents to scope')}
          >
            <Plus className="h-3.5 w-3.5" />
            {t('query.scope.add', 'Add')}
          </Button>
        )}
      </PopoverTrigger>

      <PopoverContent
        align="start"
        className="w-80 p-0"
        aria-label={t('query.scope.popover', 'Document scope selector')}
      >
        {/* Header + search */}
        <div className="px-3 pt-3 pb-2 space-y-2">
          <p className="text-sm font-semibold">
            {t('query.scope.title', 'Scope documents')}
          </p>
          <div className="relative">
            <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground pointer-events-none" />
            <Input
              placeholder={t('query.scope.searchPlaceholder', 'Search by title…')}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-8 pr-7 h-8 text-sm"
              aria-label={t('query.scope.searchLabel', 'Search documents by title')}
              aria-autocomplete="list"
              autoComplete="off"
            />
            {search && (
              <button
                type="button"
                onClick={() => setSearch('')}
                className="absolute right-2 top-2.5 text-muted-foreground hover:text-foreground"
                aria-label={t('common.clear', 'Clear')}
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>

        <Separator />

        {/* Results list */}
        <ScrollArea className="max-h-60">
          {isLoading && (
            <div className="flex items-center gap-2 px-3 py-3 text-xs text-muted-foreground">
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
              {t('common.loading', 'Loading…')}
            </div>
          )}

          {!isLoading && sortedResults.length === 0 && !search && (
            <p className="px-3 py-4 text-xs text-muted-foreground text-center">
              {t('query.scope.noDocuments', 'No completed documents in this workspace.')}
            </p>
          )}

          {!isLoading && sortedResults.length === 0 && search && (
            <p className="px-3 py-4 text-xs text-muted-foreground text-center">
              {t('query.scope.noResults', 'No documents match "{{query}}".', { query: search })}
            </p>
          )}

          <div
            role="listbox"
            aria-label={t('query.scope.resultsList', 'Document search results')}
            aria-multiselectable="true"
          >
            {sortedResults.map((item) => {
              const checked = selectedSet.has(item.id);
              return (
                <button
                  key={item.id}
                  type="button"
                  role="option"
                  aria-selected={checked}
                  onClick={() => toggle(item)}
                  className={cn(
                    'w-full flex items-center gap-2.5 px-3 py-2 text-sm',
                    'hover:bg-accent hover:text-accent-foreground',
                    'focus-visible:outline-none focus-visible:bg-accent',
                    'text-left transition-colors',
                    checked && 'bg-accent/40',
                  )}
                >
                  {/* Checkbox indicator */}
                  <span
                    className={cn(
                      'h-3.5 w-3.5 shrink-0 rounded-sm border',
                      checked
                        ? 'bg-primary border-primary flex items-center justify-center'
                        : 'border-input',
                    )}
                    aria-hidden="true"
                  >
                    {checked && (
                      <svg
                        viewBox="0 0 10 10"
                        className="h-2.5 w-2.5 text-primary-foreground"
                        fill="currentColor"
                      >
                        <path d="M1.5 5L4 7.5 8.5 2.5" stroke="currentColor" strokeWidth="1.5" fill="none" />
                      </svg>
                    )}
                  </span>
                  <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" aria-hidden="true" />
                  <span className="truncate flex-1" title={item.title}>
                    {item.title}
                  </span>
                </button>
              );
            })}
          </div>
        </ScrollArea>

        {/* Footer: count + clear */}
        {selectedIds.length > 0 && (
          <>
            <Separator />
            <div className="px-3 py-2 flex items-center justify-between">
              <span className="text-xs text-muted-foreground">
                {t('query.scope.selectedCount', '{{count}} selected', {
                  count: selectedIds.length,
                })}
              </span>
              <Button
                variant="ghost"
                size="sm"
                onClick={clearAll}
                className="h-6 text-xs gap-1 text-muted-foreground hover:text-destructive"
              >
                <X className="h-3 w-3" />
                {t('query.scope.clearAll', 'Clear all')}
              </Button>
            </div>
          </>
        )}
      </PopoverContent>
    </Popover>
  );
}
