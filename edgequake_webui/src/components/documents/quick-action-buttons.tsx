/**
 * QuickActionButtons - Row-level action buttons for document table
 *
 * @fileoverview Extracted from DocumentManager (OODA-10)
 * WHY: SRP - Row actions have distinct rendering and status logic
 *
 * Layout contract (why this file is shaped the way it is):
 *   The actions column only reserves `ROW_ACTION_BUTTON_SLOTS` (3) icon slots.
 *   A row therefore renders AT MOST three buttons: one always-useful preview
 *   action, one contextual action, and the overflow menu. The contextual slot is
 *   resolved from a single mutually-exclusive decision so a future action can
 *   never silently overflow the cell and paint over the neighbouring column.
 *
 * @module edgequake_webui/components/documents/quick-action-buttons
 */
'use client';

import { Button } from '@/components/ui/button';
import { useWorkspaceCapabilities } from '@/hooks/use-workspace-capabilities';
import { needsReuploadNotReprocess } from '@/lib/pipeline/pipeline-document-state';
import { cn } from '@/lib/utils';
import type { Document } from '@/types';
import { Eye, RefreshCw, Sparkles } from 'lucide-react';
import * as React from 'react';
import { useTranslation } from 'react-i18next';

/**
 * Statuses that allow graph explore action (completed extract only).
 * WHY: Only documents with extracted entities can be explored in graph.
 * SPEC-122: also require query_ready !== false when the serving fence is set.
 */
const GRAPH_VIEWABLE_STATUSES: readonly string[] = ['completed', 'indexed'];

function canExploreGraph(doc: Document): boolean {
  const status = doc.status ?? '';
  if (!GRAPH_VIEWABLE_STATUSES.includes(status)) return false;
  // Fence false → indexed but not serving; don't imply graph is ready.
  if (doc.query_ready === false) return false;
  return true;
}

/**
 * Statuses that show "Retry" action
 * WHY: Failed and cancelled documents can be retried/reprocessed
 */
const RETRYABLE_STATUSES: readonly string[] = ['failed', 'partial_failure', 'cancelled'];

/** The single contextual action a row may show next to Preview. */
type RowContextAction = 'retry' | 'graph' | null;

/**
 * Resolve the one contextual action for a document.
 *
 * WHY a resolver instead of inline conditionals: the actions column budgets a
 * fixed number of slots. Returning a discriminated value makes "two contextual
 * actions at once" unrepresentable, which is what keeps the cell from
 * overflowing (the old code could show Retry and Open graph together).
 */
export function resolveRowContextAction(doc: Document): RowContextAction {
  const status = doc.status ?? '';
  if (RETRYABLE_STATUSES.includes(status) && !needsReuploadNotReprocess(doc)) {
    return 'retry';
  }
  if (canExploreGraph(doc)) return 'graph';
  return null;
}

export interface QuickActionButtonsProps {
  /** Document to show actions for */
  doc: Document;
  /** Handler for "Preview" click - opens side panel */
  onPreview: (doc: Document) => void;
  /** Handler for graph explore — navigates to graph view */
  onViewInGraph: (doc: Document) => void;
  /** Handler for "Retry" click - reprocesses failed document */
  onRetry: (id: string) => void;
  /** Whether retry operation is in progress */
  isRetrying: boolean;
  /** Additional action elements (e.g., DocumentActionsMenu) */
  children?: React.ReactNode;
}

/**
 * Icon button without a tooltip wrapper.
 *
 * WHY no Radix Tooltip here: every wrapper mounted one TooltipProvider per
 * button per row (hundreds of providers in a virtualized list). The row is a
 * dense inventory surface, so `title` + `aria-label` carry the affordance at a
 * fraction of the cost.
 */
interface ActionButtonProps {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  className?: string;
}

function ActionButton({ icon, label, onClick, className }: ActionButtonProps) {
  return (
    <Button
      variant="ghost"
      size="icon-sm"
      className={className}
      onClick={onClick}
      aria-label={label}
      title={label}
    >
      {icon}
    </Button>
  );
}

/**
 * QuickActionButtons - Row-level document actions
 *
 * Renders at most three buttons, matching the actions column budget:
 * - Preview: always (opens the side panel)
 * - Contextual: Retry for retryable docs, Open graph for serving-ready docs
 * - Overflow menu (`children`)
 *
 * The quick actions stay visually quiet until the row is hovered or focused
 * (F-DOC-06). They remain keyboard and touch reachable, so the reveal is never
 * mouse-only.
 */
export function QuickActionButtons({
  doc,
  onPreview,
  onViewInGraph,
  onRetry,
  isRetrying,
  children,
}: QuickActionButtonsProps) {
  const { t } = useTranslation();
  const hasGraph = useWorkspaceCapabilities().shows_graph_ui;
  const contextAction = resolveRowContextAction(doc);
  const previewLabel = t('documents.table.preview', 'Preview');
  const graphLabel = t('documents.table.openGraph', 'Open graph');
  const retryLabel = t('documents.table.retry', 'Retry');

  return (
    <div className="flex items-center justify-end gap-0.5">
      <div
        // Reveal-on-hover keeps the row visually quiet (F-DOC-06), but it must
        // never be the ONLY way in:
        //   • group-focus-within → keyboard users tabbing into the cluster
        //   • group-has-[:focus-visible] → a row focused via keyboard
        //   • pointer-coarse → touch devices have no hover at all, so the
        //     buttons stay visible there (otherwise Preview/Retry are
        //     unreachable on phones and tablets)
        className={cn(
          'flex items-center gap-0.5 transition-opacity duration-150',
          'opacity-0 group-focus-within/row:opacity-100 group-hover/row:opacity-100',
          'group-has-[:focus-visible]/row:opacity-100',
          'pointer-coarse:opacity-100',
        )}
        data-testid="row-quick-actions"
      >
        <ActionButton
          icon={<Eye className="h-4 w-4" />}
          label={previewLabel}
          onClick={() => onPreview(doc)}
        />

        {contextAction === 'graph' && hasGraph ? (
          <ActionButton
            icon={<Sparkles className="h-4 w-4" />}
            label={graphLabel}
            onClick={() => onViewInGraph(doc)}
          />
        ) : null}

        {contextAction === 'retry' ? (
          <ActionButton
            icon={<RefreshCw className={isRetrying ? 'h-4 w-4 animate-spin' : 'h-4 w-4'} />}
            label={retryLabel}
            onClick={() => onRetry(doc.id)}
            // Uses the same destructive/orange affordance as the status badge
            // family instead of a one-off orange that matched nothing.
            className="text-orange-700 hover:bg-orange-50 hover:text-orange-800 dark:text-orange-400 dark:hover:bg-orange-950/40"
          />
        ) : null}
      </div>

      {/* Overflow menu — always visible so the row always exposes an affordance. */}
      {children}
    </div>
  );
}

export default QuickActionButtons;
