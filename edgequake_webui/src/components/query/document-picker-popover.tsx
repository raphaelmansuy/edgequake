'use client';
/**
 * @module DocumentPickerPopover
 * @description Polished, accessible document scope picker.
 *
 * Key fixes applied:
 *  - `overflow-hidden` on PopoverContent so rounded corners clip ALL child
 *    content (prevents items leaking outside the card).
 *  - Adaptive max-height via `--radix-popover-content-available-height` CSS
 *    variable — the list never overflows the viewport when the popover opens
 *    upward from the bottom of the screen.
 *  - Full keyboard navigation: ArrowUp/Down, Home/End, Enter/Space to toggle,
 *    Escape to close, ArrowDown from search jumps to first item.
 *  - ARIA: listbox + option roles, aria-selected, live region for count.
 *
 * @implements SPEC-031: Document scope selection
 */

import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from '@/components/ui/popover';
import { Separator } from '@/components/ui/separator';
import { useDocumentSearch } from '@/hooks/use-document-search';
import { cn } from '@/lib/utils';
import type { DocumentSearchItem } from '@/types';
import {
    Check,
    FileText,
    Loader2,
    Plus,
    Search,
    X,
} from 'lucide-react';
import {
    useCallback,
    useEffect,
    useMemo,
    useRef,
    useState,
    type KeyboardEvent,
} from 'react';
import { useTranslation } from 'react-i18next';

export interface DocumentPickerPopoverProps {
  selectedIds: string[];
  onSelectionChange: (ids: string[]) => void;
  disabled?: boolean;
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
  const listRef = useRef<HTMLDivElement>(null);

  const { data: results, isLoading } = useDocumentSearch(search, open);

  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  // Auto-focus search on open
  useEffect(() => {
    if (open) {
      const t = setTimeout(() => searchRef.current?.focus(), 60);
      return () => clearTimeout(t);
    }
  }, [open]);

  const toggle = useCallback(
    (item: DocumentSearchItem) => {
      onSelectionChange(
        selectedSet.has(item.id)
          ? selectedIds.filter((id) => id !== item.id)
          : [...selectedIds, item.id],
      );
    },
    [selectedIds, selectedSet, onSelectionChange],
  );

  const clearAll = useCallback(() => onSelectionChange([]), [onSelectionChange]);

  const handleOpenChange = (next: boolean) => {
    setOpen(next);
    if (!next) setSearch('');
  };

  // Selected first, then alphabetical
  const sortedResults = useMemo(() => {
    return [...results].sort((a, b) => {
      const aS = selectedSet.has(a.id) ? 0 : 1;
      const bS = selectedSet.has(b.id) ? 0 : 1;
      if (aS !== bS) return aS - bS;
      return a.title.localeCompare(b.title);
    });
  }, [results, selectedSet]);

  // ── Keyboard helpers ──────────────────────────────────────────────────────

  /** Returns all focusable option elements in the list. */
  const getOptions = (): HTMLElement[] =>
    Array.from(
      listRef.current?.querySelectorAll<HTMLElement>('[role="option"]') ?? [],
    );

  /** Move list focus by delta (-1 / +1). */
  const moveFocus = (current: HTMLElement, delta: number) => {
    const opts = getOptions();
    const idx = opts.indexOf(current);
    const next = opts[Math.max(0, Math.min(opts.length - 1, idx + delta))];
    next?.focus();
  };

  const handleSearchKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      getOptions()[0]?.focus();
    }
  };

  const handleItemKeyDown = (
    e: KeyboardEvent<HTMLButtonElement>,
    item: DocumentSearchItem,
  ) => {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        moveFocus(e.currentTarget, 1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        if (getOptions().indexOf(e.currentTarget) === 0) {
          searchRef.current?.focus();
        } else {
          moveFocus(e.currentTarget, -1);
        }
        break;
      case 'Home':
        e.preventDefault();
        getOptions()[0]?.focus();
        break;
      case 'End': {
        e.preventDefault();
        const opts = getOptions();
        opts[opts.length - 1]?.focus();
        break;
      }
      case 'Enter':
      case ' ':
        e.preventDefault();
        toggle(item);
        break;
    }
  };

  // ── Render ────────────────────────────────────────────────────────────────

  const countLabel =
    !isLoading && sortedResults.length > 0
      ? t('query.scope.resultCount', '{{count}} documents', {
          count: sortedResults.length,
        })
      : null;

  return (
    <Popover open={open} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>
        {trigger ?? (
          <button
            type="button"
            disabled={disabled}
            className={cn(
              'inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs',
              'text-muted-foreground hover:text-foreground hover:bg-muted/60',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40',
              'transition-colors',
            )}
            aria-label={t('query.scope.addDocuments', 'Add documents to scope')}
          >
            <Plus className="h-3.5 w-3.5" />
            {t('query.scope.add', 'Add')}
          </button>
        )}
      </PopoverTrigger>

      {/*
        overflow-hidden: clips ScrollArea content to the rounded card border —
        prevents items leaking outside the white card when list is long.

        z-[9999]: ensures popover renders above ALL stacking contexts including
        the messages ScrollArea.

        The inline style sets max-height using Radix's CSS variable so the card
        never exceeds available viewport space (important when opening upward
        from the bottom of the page).
      */}
      <PopoverContent
        align="start"
        sideOffset={6}
        collisionPadding={12}
        className="w-[340px] p-0 shadow-xl z-[9999] overflow-hidden"
        aria-label={t('query.scope.popover', 'Document scope selector')}
        style={{
          maxHeight:
            'calc(var(--radix-popover-content-available-height, 500px) - 24px)',
        }}
      >
        <div className="flex flex-col h-full">
          {/* ── Header ─────────────────────────────────────── */}
          <div className="px-3 pt-3 pb-2.5 shrink-0">
            <div className="flex items-center justify-between mb-2">
              <p className="text-sm font-semibold leading-none">
                {t('query.scope.title', 'Scope documents')}
              </p>
              {countLabel && (
                <span
                  className="text-xs text-muted-foreground tabular-nums"
                  aria-live="polite"
                  aria-atomic="true"
                >
                  {countLabel}
                </span>
              )}
            </div>

            {/* Search */}
            <div className="relative">
              <Search
                className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground pointer-events-none"
                aria-hidden="true"
              />
              <input
                ref={searchRef}
                type="text"
                role="searchbox"
                placeholder={t('query.scope.searchPlaceholder', 'Search by title…')}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                onKeyDown={handleSearchKeyDown}
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
                aria-activedescendant={undefined}
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
                  className="absolute right-2 top-1/2 -translate-y-1/2 rounded-sm text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-1"
                  aria-label={t('common.clearSearch', 'Clear search')}
                  tabIndex={0}
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
          </div>

          <Separator className="shrink-0" />

          {/* ── List — flex-1 so it fills remaining height ── */}
          <div
            ref={listRef}
            id="scope-picker-list"
            role="listbox"
            aria-label={t('query.scope.resultsList', 'Document search results')}
            aria-multiselectable="true"
            className="flex-1 overflow-y-auto overscroll-contain min-h-0"
          >
            {isLoading && (
              <div className="flex items-center gap-2 px-3 py-3 text-xs text-muted-foreground">
                <Loader2 className="h-3.5 w-3.5 animate-spin shrink-0" />
                {t('common.loading', 'Loading…')}
              </div>
            )}

            {!isLoading && sortedResults.length === 0 && !search && (
              <div className="px-3 py-5 text-center">
                <p className="text-xs text-muted-foreground">
                  {t(
                    'query.scope.noDocuments',
                    'No completed documents in this workspace.',
                  )}
                </p>
              </div>
            )}

            {!isLoading && sortedResults.length === 0 && search && (
              <div className="px-3 py-5 text-center">
                <p className="text-xs text-muted-foreground">
                  {t('query.scope.noResults', 'No results for "{{query}}"', {
                    query: search,
                  })}
                </p>
              </div>
            )}

            {sortedResults.map((item) => (
              <PickerItem
                key={item.id}
                item={item}
                checked={selectedSet.has(item.id)}
                onToggle={() => toggle(item)}
                onKeyDown={(e) => handleItemKeyDown(e, item)}
              />
            ))}
          </div>

          {/* ── Footer ─────────────────────────────────────── */}
          {selectedIds.length > 0 && (
            <>
              <Separator className="shrink-0" />
              <div className="px-3 py-2 flex items-center justify-between gap-2 shrink-0">
                <span className="text-xs text-muted-foreground">
                  {t('query.scope.selectedCount', '{{count}} selected', {
                    count: selectedIds.length,
                  })}
                </span>
                <button
                  type="button"
                  onClick={clearAll}
                  className={cn(
                    'text-xs text-muted-foreground hover:text-destructive',
                    'flex items-center gap-1',
                    'focus-visible:outline-none focus-visible:ring-1 rounded-sm',
                    'transition-colors',
                  )}
                >
                  <X className="h-3 w-3" aria-hidden="true" />
                  {t('query.scope.clearAll', 'Clear all')}
                </button>
              </div>
            </>
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}

// ── PickerItem ────────────────────────────────────────────────────────────────

function PickerItem({
  item,
  checked,
  onToggle,
  onKeyDown,
}: {
  item: DocumentSearchItem;
  checked: boolean;
  onToggle: () => void;
  onKeyDown: (e: KeyboardEvent<HTMLButtonElement>) => void;
}) {
  const { t } = useTranslation();

  return (
    <button
      type="button"
      role="option"
      aria-selected={checked}
      id={`scope-item-${item.id}`}
      onClick={onToggle}
      onKeyDown={onKeyDown}
      className={cn(
        'w-full flex items-center gap-2.5 px-3 py-[9px] text-sm',
        'hover:bg-accent hover:text-accent-foreground',
        'focus-visible:outline-none focus-visible:bg-accent focus-visible:text-accent-foreground',
        'text-left transition-colors duration-100 cursor-default',
        checked && 'bg-primary/5',
      )}
    >
      {/* Square checkbox */}
      <span
        className={cn(
          'flex-none h-4 w-4 shrink-0 rounded-[3px] border-2',
          'flex items-center justify-center transition-colors duration-100',
          checked
            ? 'bg-primary border-primary'
            : 'border-border/70 bg-background',
        )}
        aria-hidden="true"
      >
        {checked && (
          <Check className="h-[9px] w-[9px] text-primary-foreground stroke-[3]" />
        )}
      </span>

      {/* File icon */}
      <FileText
        className={cn(
          'flex-none h-3.5 w-3.5 shrink-0',
          checked ? 'text-primary/70' : 'text-muted-foreground/70',
        )}
        aria-hidden="true"
      />

      {/* Title */}
      <span
        className={cn(
          'truncate min-w-0 flex-1',
          checked ? 'font-medium text-foreground' : 'text-foreground/80',
        )}
        title={item.title}
      >
        {item.title}
      </span>
    </button>
  );
}
