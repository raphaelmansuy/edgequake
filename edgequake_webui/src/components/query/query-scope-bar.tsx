'use client';
/**
 * @module QueryScopeBar
 * @description Always-visible scope toolbar above the query textarea.
 *
 * Renders two distinct states:
 *  - Empty (full workspace): a low-prominence "All docs ▾" affordance so
 *    users can discover and activate scope restriction without entering settings.
 *  - Active (docs selected): secondary-color pills + add/clear controls.
 *
 * Design principle: the toolbar is ALWAYS rendered so the feature is always
 * discoverable. The empty state is intentionally muted to avoid visual noise
 * while still providing a clear call-to-action.
 *
 * @implements SPEC-031: Document scope visualization + discoverability
 */

import { cn } from '@/lib/utils';
import { useScopeDocumentLabel } from '@/hooks/use-scope-document-label';
import { ChevronDown, X } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { DocumentPickerPopover } from './document-picker-popover';

const MAX_VISIBLE_PILLS = 3;
const MAX_PILL_CHARS = 22;

export interface QueryScopeBarProps {
  /** Selected document IDs */
  selectedIds: string[];
  /** Callback when selection changes */
  onSelectionChange: (ids: string[]) => void;
  /** Whether the bar is disabled (e.g., during query execution) */
  disabled?: boolean;
}

export function QueryScopeBar({
  selectedIds,
  onSelectionChange,
  disabled = false,
}: QueryScopeBarProps) {
  const { t } = useTranslation();
  const hasScope = selectedIds.length > 0;

  const visibleIds = selectedIds.slice(0, MAX_VISIBLE_PILLS);
  const hiddenCount = selectedIds.length - visibleIds.length;

  const removeId = (id: string) =>
    onSelectionChange(selectedIds.filter((sid) => sid !== id));

  const clearAll = () => onSelectionChange([]);

  return (
    <div
      role="region"
      aria-label={t('query.scope.activeScope', 'Query scope')}
      className={cn(
        'flex items-center gap-1.5 px-1 py-1 rounded-md mb-1.5',
        // Active state: tinted background to signal narrowed context
        hasScope && 'bg-muted/40',
        disabled && 'opacity-60 pointer-events-none',
      )}
    >
      {hasScope ? (
        /* ── Active state: label + pills ───────────────────────── */
        <>
          <span className="text-xs text-muted-foreground shrink-0 select-none">
            {t('query.scope.label', 'Scope:')}
          </span>

          {/* Pills list */}
          <ul
            role="list"
            className="flex items-center gap-1 flex-nowrap min-w-0 overflow-x-auto scrollbar-none"
          >
            {visibleIds.map((id) => (
              <ScopePill
                key={id}
                documentId={id}
                onRemove={() => removeId(id)}
                disabled={disabled}
              />
            ))}

            {hiddenCount > 0 && (
              <li className="shrink-0">
                <DocumentPickerPopover
                  selectedIds={selectedIds}
                  onSelectionChange={onSelectionChange}
                  disabled={disabled}
                  trigger={
                    <button
                      type="button"
                      className={cn(
                        'inline-flex items-center rounded-full px-2 py-0.5 text-xs',
                        'bg-muted text-muted-foreground hover:bg-muted/80',
                        'focus-visible:outline-none focus-visible:ring-1',
                      )}
                      aria-label={t('query.scope.moreCount', '+{{count}} more', {
                        count: hiddenCount,
                      })}
                    >
                      +{hiddenCount}
                    </button>
                  }
                />
              </li>
            )}
          </ul>

          {/* Add more */}
          <DocumentPickerPopover
            selectedIds={selectedIds}
            onSelectionChange={onSelectionChange}
            disabled={disabled}
          />

          {/* Clear all */}
          <button
            type="button"
            onClick={clearAll}
            disabled={disabled}
            className="ml-auto shrink-0 text-muted-foreground hover:text-destructive focus-visible:outline-none"
            aria-label={t('query.scope.clearAllScope', 'Clear document scope')}
            title={t('query.scope.clearAllScope', 'Clear document scope')}
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </>
      ) : (
        /* ── Empty state: always-visible "All docs" affordance ──── */
        <DocumentPickerPopover
          selectedIds={[]}
          onSelectionChange={onSelectionChange}
          disabled={disabled}
          trigger={
            <button
              type="button"
              disabled={disabled}
              className={cn(
                'inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs',
                'text-muted-foreground/70 hover:text-muted-foreground',
                'hover:bg-muted/50 focus-visible:outline-none focus-visible:ring-1',
                'transition-colors',
              )}
              aria-label={t('query.scope.allDocsLabel', 'Query scope: all workspace documents. Click to restrict.')}
              title={t('query.scope.allDocsTitle', 'Restrict query to specific documents')}
            >
              <span>{t('query.scope.allDocs', 'All docs')}</span>
              <ChevronDown className="h-3 w-3 opacity-60" aria-hidden="true" />
            </button>
          }
        />
      )}
    </div>
  );
}

/** Individual scope pill for a single document. */
function ScopePill({
  documentId,
  onRemove,
  disabled,
}: {
  documentId: string;
  onRemove: () => void;
  disabled: boolean;
}) {
  const { t } = useTranslation();
  const label = useScopeDocumentLabel(documentId);

  const displayLabel = label
    ? label.length > MAX_PILL_CHARS
      ? `${label.slice(0, MAX_PILL_CHARS)}\u2026`
      : label
    : `${documentId.slice(0, 8)}\u2026`;

  return (
    <li role="listitem" className="shrink-0">
      <span
        className={cn(
          'inline-flex items-center gap-1 rounded-full pl-2.5 pr-1 py-0.5',
          'bg-secondary text-secondary-foreground text-xs max-w-[180px]',
        )}
        title={label ?? documentId}
      >
        <span className="truncate">{displayLabel}</span>
        <button
          type="button"
          onClick={onRemove}
          disabled={disabled}
          className={cn(
            'shrink-0 rounded-full p-0.5 ml-0.5',
            'text-muted-foreground hover:text-destructive',
            'focus-visible:outline-none focus-visible:ring-1',
          )}
          aria-label={t('query.scope.removeDoc', 'Remove {{title}} from scope', {
            title: label ?? documentId,
          })}
        >
          <X className="h-3 w-3" />
        </button>
      </span>
    </li>
  );
}


