/**
 * ProcessingStatusSummary - Shows pipeline processing status
 *
 * @fileoverview Extracted from DocumentManager (OODA-11)
 * WHY: SRP - Processing status display is distinct responsibility
 *
 * @module edgequake_webui/components/documents/processing-status-summary
 */
'use client';

import type { Document, PipelineStatus } from '@/types';
import { CheckCircle, Clock, Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { isProcessingStatus, getDocumentDisplayStatus } from './status-badge';

export interface ProcessingStatusSummaryProps {
  /** Pipeline status from API query */
  pipelineStatus: PipelineStatus;
  /** Documents to show processing details for */
  documents: Document[];
  /** Callback when user clicks to see details */
  onOpenDetails: () => void;
}

/**
 * ProcessingStatusSummary - Compact processing status display
 *
 * Shows:
 * - Running/queued task count with spinner
 * - Processing stage details for active documents
 * - Completed task count
 * - Click CTA to open pipeline dialog
 *
 * Only renders when there are running or queued tasks.
 */
export function ProcessingStatusSummary({
  pipelineStatus,
  documents,
  onOpenDetails,
}: ProcessingStatusSummaryProps) {
  const { t } = useTranslation();

  // WHY: Count documents actually in processing state (not task count)
  // Tasks can be "processing" even when their documents are failed/completed
  // (e.g., after server restart: orphan recovery fails docs but tasks keep running)
  const processingDocCount = documents?.filter(
    (d) => isProcessingStatus(getDocumentDisplayStatus(d))
  ).length ?? 0;

  const queuedDocCount = documents?.filter(
    (d) => getDocumentDisplayStatus(d) === 'queued'
  ).length ?? 0;

  const activeDocCount = processingDocCount;

  // Filter documents with stage details (for progress detail display)
  const processingDocs = documents?.filter((d) => {
    const display = getDocumentDisplayStatus(d);
    return (
      d.current_stage &&
      isProcessingStatus(display) &&
      display !== 'queued'
    );
  }) ?? [];

  const queuedDocs = documents?.filter(
    (d) => getDocumentDisplayStatus(d) === 'queued'
  ) ?? [];

  const pipelineQueuedTasks = Math.max(
    0,
    pipelineStatus.queued_tasks - queuedDocCount,
  );

  // Show banner when documents are actively processing or queued tasks await processing
  const shouldShow =
    activeDocCount > 0 ||
    queuedDocCount > 0 ||
    pipelineStatus.queued_tasks > 0;
  if (!shouldShow) return null;

  return (
    <div
      className="flex flex-col gap-2 px-3 py-2 bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 rounded-lg cursor-pointer hover:bg-blue-100 dark:hover:bg-blue-950/50 transition-colors"
      onClick={onOpenDetails}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => e.key === 'Enter' && onOpenDetails()}
    >
      <div className="flex items-center gap-4">
        <Loader2 className="h-4 w-4 text-blue-600 dark:text-blue-400 animate-spin shrink-0" />
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium text-blue-700 dark:text-blue-300">
            {processingDocCount > 0 && (queuedDocCount > 0 || pipelineQueuedTasks > 0)
              ? t(
                  'pipeline.processingAndQueued',
                  '{{processing}} processing · {{queued}} queued',
                  {
                    processing: processingDocCount - queuedDocCount,
                    queued: queuedDocCount + pipelineQueuedTasks,
                  },
                )
              : processingDocCount > 0
                ? t('pipeline.processing', 'Processing {{count}} document(s)', {
                    count: processingDocCount,
                  })
                : t('pipeline.queued', '{{count}} document(s) queued', {
                    count: queuedDocCount + pipelineQueuedTasks,
                  })}
          </p>
          {/* Show detailed stage messages for processing documents */}
          {processingDocs.length > 0 && (
            <div className="mt-1 space-y-0.5">
              {processingDocs.slice(0, 2).map((doc) => (
                <p
                  key={doc.id}
                  className="text-xs text-blue-600 dark:text-blue-400 truncate"
                >
                  {doc.title || doc.file_name || 'Document'}:{' '}
                  {doc.stage_message || doc.current_stage || 'Processing...'}
                </p>
              ))}
            </div>
          )}
          {queuedDocs.length > 0 && (
            <div className="mt-1 space-y-0.5">
              {queuedDocs.slice(0, 2).map((doc) => (
                <p
                  key={doc.id}
                  className="text-xs text-amber-700 dark:text-amber-400 truncate"
                >
                  {doc.title || doc.file_name || 'Document'}:{' '}
                  {doc.stage_message ||
                    t(
                      'pipeline.waitingForSlot',
                      'Waiting for a processing slot',
                    )}
                </p>
              ))}
            </div>
          )}
        </div>
        <div className="flex items-center gap-3 text-xs text-blue-600 dark:text-blue-400">
          {(queuedDocCount > 0 || pipelineQueuedTasks > 0) &&
            processingDocCount > queuedDocCount && (
              <span className="flex items-center gap-1">
                <Clock className="h-3 w-3" />
                {queuedDocCount + pipelineQueuedTasks} queued
              </span>
            )}
          {pipelineStatus.completed_tasks > 0 && (
            <span className="flex items-center gap-1">
              <CheckCircle className="h-3 w-3 text-green-600" />
              {pipelineStatus.completed_tasks} done
            </span>
          )}
          <span className="text-blue-500">Click for details →</span>
        </div>
      </div>
    </div>
  );
}

export default ProcessingStatusSummary;
