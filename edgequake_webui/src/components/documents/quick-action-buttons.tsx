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
import type { Document } from '@/types';
import { ExternalLink, Eye, RefreshCw, Sparkles } from 'lucide-react';
import * as React from 'react';

/**
 * Statuses that allow "View in Graph" action
 * WHY: Only documents with extracted entities can be viewed in graph
 */
const GRAPH_VIEWABLE_STATUSES: readonly string[] = ['completed', 'indexed'];

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
  /** Handler for "View in Graph" click - navigates to graph view */
  onViewInGraph: (doc: Document) => void;
  /** Handler for "Retry" click - reprocesses failed document */
  onRetry: (id: string) => void;
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
}

function ActionButton({ icon, label, onClick, className }: ActionButtonProps) {
  return (
    <TooltipProvider delayDuration={300}>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className={`h-8 w-8 ${className || ''}`}
            onClick={onClick}
            aria-label={label}
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
 * - View in Graph: Only for completed/indexed documents
 * - Retry: Only for failed/partial_failure documents
 */
export function QuickActionButtons({
  doc,
  onViewDetails,
  onPreview,
  onViewInGraph,
  onRetry,
  isRetrying,
  children,
}: QuickActionButtonsProps) {
  const status = doc.status ?? '';
  const canViewInGraph = GRAPH_VIEWABLE_STATUSES.includes(status);
  const canRetry = RETRYABLE_STATUSES.includes(status);

  return (
    <div className="flex items-center gap-1 justify-end">
      {/* View Details - navigates to document detail page */}
      <ActionButton
        icon={<ExternalLink className="h-4 w-4" />}
        label="View Details"
        onClick={() => onViewDetails(doc)}
      />

      {/* Preview - opens side panel */}
      <ActionButton
        icon={<Eye className="h-4 w-4" />}
        label="Preview"
        onClick={() => onPreview(doc)}
      />

      {/* View in Graph - only for completed documents */}
      {canViewInGraph && (
        <ActionButton
          icon={<Sparkles className="h-4 w-4" />}
          label="View in Graph"
          onClick={() => onViewInGraph(doc)}
        />
      )}

      {/* Retry - only for failed documents */}
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

      {/* Additional actions (e.g., dropdown menu) */}
      {children}
    </div>
  );
}

export default QuickActionButtons;
