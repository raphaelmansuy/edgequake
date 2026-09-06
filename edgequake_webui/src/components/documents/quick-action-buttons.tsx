/**
 * QuickActionButtons - Row-level action buttons for document table
 *
 * @fileoverview Extracted from DocumentManager (OODA-10)
 * WHY: SRP - Row actions have distinct rendering and status logic
 *
 * @module edgequake_webui/components/documents/quick-action-buttons
 */
'use client';

import { Button } from '@/components/ui/button';
import {
    Tooltip,
    TooltipContent,
    TooltipProvider,
    TooltipTrigger,
} from '@/components/ui/tooltip';
import { needsReuploadNotReprocess } from '@/lib/pipeline/pipeline-document-state';
import { useCanSetLabels } from '@/hooks/use-can-set-labels';
import type { Document } from '@/types';
import { ExternalLink, Eye, RefreshCw, Sparkles } from 'lucide-react';
import * as React from 'react';

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

export interface QuickActionButtonsProps {
  /** Document to show actions for */
  doc: Document;
  /** Handler for "View Details" click - navigates to detail page */
  onViewDetails: (doc: Document) => void;
  /** Handler for "Preview" click - opens side panel */
  onPreview: (doc: Document) => void;
  /** Handler for graph explore — navigates to graph view */
  onViewInGraph: (doc: Document) => void;
  /** Handler for "Retry" click - reprocesses failed document */
  onRetry: (id: string) => void;
  /** SPEC-146: retry quarantined labels via PATCH security-labels */
  onRetryLabels?: (doc: Document) => void;
  /** Whether retry operation is in progress */
  isRetrying: boolean;
  /** Additional action elements (e.g., DocumentActionsMenu) */
  children?: React.ReactNode;
}

/**
 * Individual action button with tooltip
 */
interface ActionButtonProps {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
  className?: string;
  testId?: string;
}

function ActionButton({ icon, label, onClick, className, testId }: ActionButtonProps) {
  return (
    <TooltipProvider delayDuration={300}>
      <Tooltip delayDuration={300}>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className={`h-8 w-8 ${className || ''}`}
            onClick={onClick}
            aria-label={label}
            data-testid={testId}
          >
            {icon}
          </Button>
        </TooltipTrigger>
        <TooltipContent>{label}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

/**
 * QuickActionButtons - Row-level document actions
 *
 * Renders action buttons based on document status:
 * - View Details: Always visible
 * - Preview: Always visible
 * - Open graph: completed/indexed and not fence-blocked (query_ready !== false)
 * - Retry: Only for failed/partial_failure documents
 */
export function QuickActionButtons({
  doc,
  onViewDetails,
  onPreview,
  onViewInGraph,
  onRetry,
  onRetryLabels,
  isRetrying,
  children,
}: QuickActionButtonsProps) {
  const canSetLabels = useCanSetLabels();
  const status = doc.status ?? '';
  const canViewInGraph = canExploreGraph(doc);
  // Orphan staging shells need dismiss + re-upload, not Retry/reprocess.
  const canRetry =
    RETRYABLE_STATUSES.includes(status) && !needsReuploadNotReprocess(doc);
  const canRetryLabels =
    doc.security_status === 'quarantined' &&
    Boolean(onRetryLabels) &&
    canSetLabels;

  return (
    <div className="flex items-center gap-1 justify-end">
      {/* Retry labels stays keyboard-visible when quarantined (not hover-only). */}
      {canRetryLabels ? (
        <ActionButton
          icon={
            <RefreshCw
              className={`h-4 w-4 ${isRetrying ? 'animate-spin' : ''}`}
            />
          }
          label="Retry labels"
          onClick={() => onRetryLabels?.(doc)}
          className="text-amber-700 hover:text-amber-800 hover:bg-amber-50"
          testId="spec146-retry-labels"
        />
      ) : null}

      {/* Other actions — visible on row hover for a cleaner table (F-DOC-06) */}
      <div className="flex items-center gap-0.5 opacity-0 group-hover/row:opacity-100 focus-within:opacity-100 transition-opacity duration-150">
        <ActionButton
          icon={<ExternalLink className="h-4 w-4" />}
          label="View Details"
          onClick={() => onViewDetails(doc)}
        />

        <ActionButton
          icon={<Eye className="h-4 w-4" />}
          label="Preview"
          onClick={() => onPreview(doc)}
        />

        {canViewInGraph && (
          <ActionButton
            icon={<Sparkles className="h-4 w-4" />}
            label="Open graph"
            onClick={() => onViewInGraph(doc)}
          />
        )}

        {canRetry && (
          <ActionButton
            icon={
              <RefreshCw
                className={`h-4 w-4 ${isRetrying ? 'animate-spin' : ''}`}
              />
            }
            label="Retry"
            onClick={() => onRetry(doc.id)}
            className="text-orange-600 hover:text-orange-700 hover:bg-orange-50"
          />
        )}
      </div>

      {children}
    </div>
  );
}

export default QuickActionButtons;
