'use client';
/**
 * @module DocumentPickerPopover
 * @description Polished popover for selecting documents to scope a query.
 *
 * Polish principles:
 * - Square checkboxes (not circles) for clear multi-select semantics
 * - Auto-focus search input on open for immediate keyboard interaction
 * - Result count in header ("8 documents", "3 of 42")
 * - Selected items float to top with stronger highlight
 * - Keyboard navigation: Escape closes, Enter selects focused item
 * - Smooth hover/focus transitions with clear active states
 * - Footer shows selection summary with inline clear
 *
 * @implements SPEC-031: Document scope selection
 */

import { Button } from '@/components/ui/button';
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { useDocumentSearch } from '@/hooks/use-document-search';
import { cn } from '@/lib/utils';
import type { DocumentSearchItem } from '@/types';
import { Check, FileText, Loader2, Plus, Search, X } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
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
  const searchRef = useRef<HTMLInputElement>(null);

  const { data: results, isLoading } = useDocumentSearch(search, open);

  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  // Auto-focus search input when popover opens
  useEffect(() => {
    if (open) {
      const timer = setTimeout(() => searchRef.current?.focus(), 50);
      return () => clearTimeout(timer);
    }
  }, [open]);

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

  const clearAll = useCallback(() => onSelectionChange([]), [onSelectionChange]);

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

  // Count label: "8 documents" or "3 of 42" when search active
  const countLabel = useMemo(() => {
    if (isLoading) return null;
    if (sortedResults.length === 0) return null;
    return t('query.scope.resultCount', '{{count}} documents', {
      count: sortedResults.length,
    });
  }, [sortedResults.length, isLoading, t]);

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
        sideOffset={6}
        className="w-[340px] p-0 shadow-xl z-[9999]"
        aria-label={t('query.scope.popover', 'Document scope selector')}
        onKeyDown={(e) => {
          if (e.key === 'Escape') handleOpenChange(false);
        }}
      >
        {/* ── Header ─────────────────────────────────────────── */}
        <div className="px-3 pt-3 pb-2.5">
          <div className="flex items-center justify-between mb-2">
            <p className="text-sm font-semibold leading-none">
              {t('query.scope.title', 'Scope documents')}
            </p>
            {countLabel && (
              <span className="text-xs text-muted-foreground tabular-nums">
                {countLabel}
              </span>
            )}
          </div>

          {/* Search input */}
          <div className="relative">
            <Search
              className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground pointer-events-none"
              aria-hidden="true"
            />
            <input
              ref={searchRef}
              type="text"
              placeholder={t('query.scope.searchPlaceholder', 'Search by title…')}
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className={cn(
                'w-full h-8 pl-8 pr-7 text-sm rounded-md',
                'bg-muted/50 border-0 ring-1 ring-border/60',
                'placeholder:text-muted-foreground/60',
                'focus:outline-none focus:ring-2 focus:ring-primary/40',
                'transition-shadow',
              )}
              aria-label={t('query.scope.searchLabel', 'Search documents by title')}
              aria-autocomplete="list"
              aria-controls="scope-picker-list"
              autoComplete="off"
              spellCheck={false}
            />
            {search && (
              <button
                type="button"
                onClick={() => {
                  setSearch('');
                  searchRef.current?.focus();
                }}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground rounded-sm focus-visible:outline-none focus-visible:ring-1"
                aria-label={t('common.clear', 'Clear search')}
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        </div>

        <Separator />

        {/* ── Results list ────────────────────────────────────── */}
        <ScrollArea className="max-h-[260px]">
          {isLoading && (
            <div className="flex items-center gap-2 px-3 py-3 text-xs text-muted-foreground">
              <Loader2 className="h-3.5 w-3.5 animate-spin shrink-0" />
              {t('common.loading', 'Loading…')}
            </div>
          )}

          {!isLoading && sortedResults.length === 0 && !search && (
            <div className="px-3 py-5 text-center">
              <p className="text-xs text-muted-foreground">
                {t('query.scope.noDocuments', 'No completed documents in this workspace.')}
              </p>
            </div>
          )}

          {!isLoading && sortedResults.length === 0 && search && (
            <div className="px-3 py-5 text-center">
              <p className="text-xs text-muted-foreground">
                {t('query.scope.noResults', 'No results for "{{query}}"', { query: search })}
              </p>
            </div>
          )}

          {sortedResults.length > 0 && (
            <div
              id="scope-picker-list"
              role="listbox"
              aria-label={t('query.scope.resultsList', 'Document search results')}
              aria-multiselectable="true"
            >
              {sortedResults.map((item) => (
                <PickerItem
                  key={item.id}
                  item={item}
                  checked={selectedSet.has(item.id)}
                  onToggle={() => toggle(item)}
                />
              ))}
            </div>
          )}
        </ScrollArea>

        {/* ── Footer: selection summary ─────────────────────── */}
        {selectedIds.length > 0 && (
          <>
            <Separator />
            <div className="px-3 py-2 flex items-center justify-between gap-2">
              <span className="text-xs text-muted-foreground shrink-0">
                {t('query.scope.selectedCount', '{{count}} selected', {
                  count: selectedIds.length,
                })}
              </span>
              <button
                type="button"
                onClick={clearAll}
                className={cn(
                  'text-xs text-muted-foreground hover:text-destructive',
                  'flex items-center gap-1 shrink-0',
                  'focus-visible:outline-none focus-visible:ring-1 rounded-sm',
                  'transition-colors',
                )}
              >
                <X className="h-3 w-3" />
                {t('query.scope.clearAll', 'Clear all')}
              </button>
            </div>
          </>
        )}
      </PopoverContent>
    </Popover>
  );
}

/** Single item row in the picker list. */
function PickerItem({
  item,
  checked,
  onToggle,
}: {
  item: DocumentSearchItem;
  checked: boolean;
  onToggle: () => void;
}) {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      role="option"
      aria-selected={checked}
      onClick={onToggle}
      className={cn(
        'w-full flex items-center gap-2.5 px-3 py-2 text-sm',
        'hover:bg-accent hover:text-accent-foreground',
        'focus-visible:outline-none focus-visible:bg-accent',
        'text-left transition-colors duration-100',
        checked && 'bg-primary/5',
      )}
    >
      {/* ── Square checkbox ─────────────────────────────────── */}
      <span
        className={cn(
          'flex-none h-4 w-4 rounded-[3px] border-2 transition-colors duration-100',
          'flex items-center justify-center',
          checked
            ? 'bg-primary border-primary'
            : 'border-border bg-background group-hover:border-primary/50',
        )}
        aria-hidden="true"
      >
        {checked && (
          <Check
            className="h-2.5 w-2.5 text-primary-foreground stroke-[2.5]"
            aria-hidden="true"
          />
        )}
      </span>

      {/* ── File icon + title ─────────────────────────────── */}
      <FileText
        className={cn(
          'flex-none h-3.5 w-3.5',
          checked ? 'text-primary/70' : 'text-muted-foreground',
        )}
        aria-hidden="true"
      />
      <span
        className={cn(
          'truncate flex-1 min-w-0',
          checked && 'font-medium',
        )}
        title={item.title}
      >
        {item.title}
      </span>
    </button>
  );
}
