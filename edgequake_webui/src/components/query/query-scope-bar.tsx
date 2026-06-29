'use client';
/**
 * @module QueryScopeBar
 * @description Always-visible scope toolbar above the query textarea.
 *
 * Two states:
 *  - Empty: low-prominence "All docs ▾" button — the discoverability affordance.
 *    Deliberately subtle so it doesn't compete with the input, but always present.
 *  - Active: secondary-color pills with × dismiss + Add / Clear All controls.
 *
 * Polish details:
 *  - `group` on the pill span so the × button reveals on hover
 *  - Pills use `ring-1` border for crisper look than background-only
 *  - "All docs" trigger has a filter icon for semantic clarity
 *  - Smooth transitions throughout
 *
 * @implements SPEC-031: Document scope visualization + discoverability
 */

import { useScopeDocumentLabel } from '@/hooks/use-scope-document-label';
import { cn } from '@/lib/utils';
import { ChevronDown, Filter, X } from 'lucide-react';
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
        'flex items-center gap-1.5 px-1 py-1 rounded-md mb-1.5 min-h-[30px]',
        'transition-colors duration-150',
        hasScope && 'bg-primary/5 ring-1 ring-primary/10',
        disabled && 'opacity-60 pointer-events-none',
      )}
    >
      {hasScope ? (
        /* ── Active state ──────────────────────────────────────── */
        <>
          <span className="text-[11px] font-medium text-muted-foreground shrink-0 select-none tracking-wide uppercase">
            {t('query.scope.label', 'Scope')}
          </span>

          {/* Pills */}
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
                        'bg-primary/10 text-primary hover:bg-primary/20',
                        'ring-1 ring-primary/20 font-medium',
                        'focus-visible:outline-none focus-visible:ring-2',
                        'transition-colors',
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
            className={cn(
              'ml-auto shrink-0 rounded-sm p-0.5',
              'text-muted-foreground/60 hover:text-destructive',
              'focus-visible:outline-none focus-visible:ring-1',
              'transition-colors',
            )}
            aria-label={t('query.scope.clearAllScope', 'Clear document scope')}
            title={t('query.scope.clearAllScope', 'Clear document scope')}
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </>
      ) : (
        /* ── Empty state: discoverable "All docs" trigger ──────── */
        <DocumentPickerPopover
          selectedIds={[]}
          onSelectionChange={onSelectionChange}
          disabled={disabled}
          trigger={
            <button
              type="button"
              disabled={disabled}
              className={cn(
                'inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs',
                'text-muted-foreground/60 hover:text-muted-foreground',
                'hover:bg-muted/60',
                'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/30',
                'transition-all duration-150',
              )}
              aria-label={t(
                'query.scope.allDocsLabel',
                'Query scope: all workspace documents. Click to restrict.',
              )}
              title={t(
                'query.scope.allDocsTitle',
                'Restrict query to specific documents',
              )}
            >
              <Filter className="h-3 w-3 opacity-70" aria-hidden="true" />
              <span>{t('query.scope.allDocs', 'All docs')}</span>
              <ChevronDown className="h-3 w-3 opacity-50" aria-hidden="true" />
            </button>
          }
        />
      )}
    </div>
  );
}

/** Individual scope pill. Shows full title on hover via `title` attribute. */
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
          'group inline-flex items-center gap-1 rounded-full',
          'pl-2.5 pr-1.5 py-0.5 text-xs font-medium',
          'bg-primary/10 text-primary',
          'ring-1 ring-primary/20',
          'max-w-[190px]',
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
            'text-primary/50 hover:text-destructive hover:bg-destructive/10',
            'focus-visible:outline-none focus-visible:ring-1',
            'transition-colors',
          )}
          aria-label={t('query.scope.removeDoc', 'Remove {{title}} from scope', {
            title: label ?? documentId,
          })}
        >
          <X className="h-2.5 w-2.5" />
        </button>
      </span>
    </li>
  );
}

