/**
 * @module DocumentTableRow
 * @description Single document row in the documents table.
 * Extracted from DocumentManager for SRP compliance (OODA-15).
 * 
 * WHY: Table row rendering was inline in DocumentManager, violating SRP.
 * This component:
 * - Handles row selection and highlighting
 * - Displays document metadata with file type icons
 * - Shows status badges and error messages
 * - Provides quick actions and context menu
 * 
 * @implements FEAT0004 - Processing status tracking per document
 * @implements FEAT0602 - Real-time progress indicators
 */
'use client';

import { Checkbox } from '@/components/ui/checkbox';
import { TableCell, TableRow } from '@/components/ui/table';
import { cn } from '@/lib/utils';
import { getDocumentDisplayStatus } from '@/lib/documents/status-domain';
import {
  buildIngestionRunView,
  formatRunHeadline,
} from '@/lib/pipeline/ingestion-run-view';
import {
    getEffectiveErrorMessage,
    isTerminalFailureDocument,
} from '@/lib/utils/document-status';
import type { Document } from '@/types';
import { formatDistanceToNow } from 'date-fns';
import {
    File,
    FileCode,
    FileImage,
    FileSpreadsheet,
    FileText,
    FileType,
} from 'lucide-react';
import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { CostCell } from './cost-cell';
import { DocumentActionsMenu } from './document-actions-menu';
import { EnhancedStatusBadge } from './enhanced-status-badge';
import { ErrorMessagePopover } from './error-message-popover';
import { QuickActionButtons } from './quick-action-buttons';
import { ClassificationChip } from '@/components/security/classification-chip';
import { ShareModeBadge } from '@/components/security/share-mode-badge';
import { principalDisplayName } from '@/components/security/principal-select';
import { Badge } from '@/components/ui/badge';

/** Stages that show live backend detail under the status badge */
const LIVE_STAGE_MESSAGE_STAGES = new Set([
  'converting',
  'storing',
  'indexing',
  'embedding',
  'extracting',
  'chunking',
  'preprocessing',
  'gleaning',
  'merging',
  'summarizing',
  'processing',
  'queued',
]);

/**
 * Get file type icon and color based on file extension.
 * WHY: Visual distinction helps users quickly identify document types.
 */
function getFileTypeIcon(fileName: string | undefined | null) {
  if (!fileName) return { icon: File, color: 'text-muted-foreground' };
  const ext = fileName.split('.').pop()?.toLowerCase();
  switch (ext) {
    case 'pdf':
      return { icon: FileText, color: 'text-red-500' };
    case 'doc':
    case 'docx':
      return { icon: FileType, color: 'text-blue-500' };
    case 'xls':
    case 'xlsx':
    case 'csv':
      return { icon: FileSpreadsheet, color: 'text-green-500' };
    case 'md':
    case 'markdown':
      return { icon: FileCode, color: 'text-purple-500' };
    case 'txt':
      return { icon: FileText, color: 'text-gray-500' };
    case 'html':
    case 'htm':
    case 'json':
    case 'xml':
      return { icon: FileCode, color: 'text-orange-500' };
    case 'jpg':
    case 'jpeg':
    case 'png':
    case 'gif':
    case 'webp':
      return { icon: FileImage, color: 'text-pink-500' };
    default:
      return { icon: File, color: 'text-muted-foreground' };
  }
}

/**
 * Highlight search matches in text.
 * WHY: Visual feedback shows which part of title matched the search.
 */
function highlightMatches(text: string, query: string): React.ReactNode {
  if (!query.trim()) return text;
  const regex = new RegExp(
    `(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`,
    'gi'
  );
  const parts = text.split(regex);
  return parts.map((part, i) =>
    regex.test(part) ? (
      <mark
        key={i}
        className="bg-yellow-200 dark:bg-yellow-700 px-0.5 rounded"
      >
        {part}
      </mark>
    ) : (
      part
    )
  );
}

/**
 * Props for DocumentTableRow component.
 */
export interface DocumentTableRowProps {
  /** Document to display */
  doc: Document;
  /** Row index (for alternating colors) */
  index: number;
  /** Whether this row is selected (checkbox) */
  isSelected: boolean;
  /** Whether this row is the active preview document */
  isActive: boolean;
  /** Dim completed/idle rows while another document is actively ingesting */
  isBackground?: boolean;
  /** LAW-IS3: when ActiveRuns owns this doc, hide row stage subtitle */
  isLiveRun?: boolean;
  /** Current search query for highlighting */
  searchQuery: string;
  /** Called when selection checkbox changes */
  onSelect: (docId: string, checked: boolean) => void;
  /** Called when row is clicked (single click) */
  onClick: (doc: Document) => void;
  /** Called when row is double-clicked */
  onDoubleClick: (doc: Document) => void;
  /** Called when View Details action is triggered */
  onViewDetails: (doc: Document) => void;
  /** Called when View in Graph action is triggered */
  onViewInGraph: (doc: Document) => void;
  /** Called when View PDF action is triggered */
  onViewPdf: (doc: Document) => void;
  /** Called when Retry action is triggered (failed-doc quick retry, no dialog) */
  onRetry: (docId: string) => void;
  /** SPEC-146: retry quarantined security labels */
  onRetryLabels?: (doc: Document) => void;
  /** Username lookup for owner_principal_id */
  ownerNames?: Record<string, string>;
  /** Called when Reprocess action is triggered (opens the choice dialog) */
  onReprocess: (docId: string) => void;
  /** Called when Cancel action is triggered */
  onCancel: (trackId: string) => void;
  /** Called when Delete action is triggered */
  onDelete: (docId: string) => void;
  /** Whether a retry operation is pending */
  isRetrying: boolean;
  /** Whether a cancel operation is pending */
  isCancelling: boolean;
  /**
   * Whether a delete operation is currently in progress for this document.
   * SPEC-050: Dims the row and shows "Deleting" badge.
   */
  isDeleting?: boolean;
  /** SPEC-099: Cost column opt-in */
  showCostColumn?: boolean;
  /** SPEC-146: Class / Share columns when DOC_ABAC is on */
  showAbacColumns?: boolean;
}

/**
 * Single document row in the documents table.
 * Memoized for performance with large document lists.
 */
export const DocumentTableRow = memo(function DocumentTableRow({
  doc,
  index,
  isSelected,
  isActive,
  isBackground = false,
  isLiveRun = false,
  searchQuery,
  onSelect,
  onClick,
  onDoubleClick,
  onViewDetails,
  onViewInGraph,
  onViewPdf,
  onRetry,
  onRetryLabels,
  ownerNames,
  onReprocess,
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
  isDeleting = false,
  showCostColumn = false,
  showAbacColumns = false,
}: DocumentTableRowProps) {
  const { t } = useTranslation();
  const displayStatus = getDocumentDisplayStatus(doc);

  // WHY: Visual distinction for document status
  const rowClassName = cn(
    'cursor-pointer transition-colors duration-150',
    'hover:bg-primary/5 dark:hover:bg-primary/10',
    isActive && 'bg-primary/10 dark:bg-primary/15 ring-1 ring-primary/20',
    index % 2 === 0 ? 'bg-background' : 'bg-muted/20',
    // SPEC-048: only gently de-emphasize non-active rows — never look disabled
    isBackground && 'opacity-80',
    // SPEC-050: Dim row while deletion is in progress
    isDeleting && 'opacity-50 pointer-events-none',
    // SPEC-099 F-099-15: highlight via domain display status (covers delete_failed)
    displayStatus === 'failed' &&
      'bg-red-50/50 dark:bg-red-950/20 border-l-4 border-l-red-500',
    displayStatus === 'delete_failed' &&
      'bg-rose-50/50 dark:bg-rose-950/20 border-l-4 border-l-rose-500',
    displayStatus === 'partial_failure' &&
      'bg-orange-50/50 dark:bg-orange-950/20 border-l-4 border-l-orange-500',
    displayStatus === 'cancelled' &&
      'bg-gray-50/50 dark:bg-gray-950/20 border-l-4 border-l-gray-400'
  );

  const { icon: FileIcon, color } = getFileTypeIcon(doc.file_name);
  const displayTitle =
    doc.title || doc.file_name || `Document ${doc.id.slice(0, 8)}`;

  return (
    <TableRow
      className={cn(rowClassName, 'group/row')}
      onClick={() => onClick(doc)}
      onDoubleClick={() => onDoubleClick(doc)}
      data-testid={`document-row-${doc.id}`}
      data-document-title={displayTitle}
    >
      {/* Selection Checkbox */}
      <TableCell className="w-11 overflow-hidden" onClick={(e) => e.stopPropagation()}>
        <Checkbox
          checked={isSelected}
          onCheckedChange={(checked) => onSelect(doc.id, !!checked)}
          aria-label={t('documents.bulk.select', 'Select')}
        />
      </TableCell>

      {/* Title with File Type Icon — max-w-0 enables truncate under table-fixed */}
      <TableCell className="max-w-0 overflow-hidden font-medium">
        <div className="flex min-w-0 flex-col gap-0.5">
          <div className="flex min-w-0 items-center gap-2">
            <FileIcon className={cn('h-4 w-4 shrink-0', color)} aria-hidden="true" />
            <span className="truncate" title={displayTitle}>
              {highlightMatches(displayTitle, searchQuery)}
            </span>
          </div>
          {/* Error message for failed/cancelled documents */}
          {isTerminalFailureDocument(doc) && getEffectiveErrorMessage(doc) && (
              <ErrorMessagePopover
                message={getEffectiveErrorMessage(doc)!}
                documentId={doc.id}
                onRetry={() => onRetry(doc.id)}
                isRetrying={isRetrying}
              />
            )}
          {/* Cancelled indicator when no error message */}
          {doc.status === 'cancelled' && !getEffectiveErrorMessage(doc) && (
            <span className="truncate text-xs text-muted-foreground">
              {t('documents.cancelled.subtitle', 'Processing was cancelled')}
            </span>
          )}
        </div>
      </TableCell>

      {/* Status Badge — SPEC-048: single RunView line (DEF-08) */}
      <TableCell className="overflow-hidden">
        <div className="flex min-w-0 flex-col gap-1">
          <div className="min-w-0 max-w-full">
            <EnhancedStatusBadge document={doc} />
          </div>
          {(() => {
            // LAW-IS3: Active View owns live narrative — table is inventory only.
            if (isLiveRun) return null;
            const run = buildIngestionRunView(doc);
            if (!run || run.stageStatus === 'complete') return null;
            if (!LIVE_STAGE_MESSAGE_STAGES.has(String(run.stage))) return null;
            return (
              <span
                className="max-w-full truncate text-xs text-muted-foreground"
                data-testid="spec048-row-stage"
                data-stage={run.stage}
                title={formatRunHeadline(run)}
              >
                {formatRunHeadline(run)}
              </span>
            );
          })()}
        </div>
      </TableCell>

      {/* SPEC-146 Class / Share / Owner — existence-hiding: never Restricted */}
      {showAbacColumns ? (
        <>
          <TableCell className="overflow-hidden" data-testid="spec146-cell-class">
            <div className="flex flex-col gap-0.5 min-w-0">
              <ClassificationChip value={doc.classification} />
              {doc.security_status === 'quarantined' ? (
                <Badge
                  variant="outline"
                  className="w-fit text-[10px] px-1 py-0 border-amber-500 text-amber-800 dark:text-amber-200"
                  data-testid="spec146-quarantine-chip"
                >
                  {t('security.quarantined', 'Labeling failed / Quarantined')}
                </Badge>
              ) : null}
            </div>
          </TableCell>
          <TableCell className="overflow-hidden" data-testid="spec146-cell-share">
            <ShareModeBadge value={doc.share_mode} />
          </TableCell>
          <TableCell
            className="overflow-hidden text-xs"
            data-testid="spec146-cell-owner"
          >
            <span className="truncate block">
              {doc.owner_principal_id
                ? ownerNames?.[doc.owner_principal_id] ||
                  principalDisplayName(undefined, doc.owner_principal_id)
                : '—'}
            </span>
          </TableCell>
        </>
      ) : null}

      {/* Entity Count */}
      <TableCell className="overflow-hidden text-center tabular-nums">
        {doc.entity_count ?? doc.chunk_count ?? '-'}
      </TableCell>

      {/* Cost — SPEC-099: only when showCostColumn */}
      {showCostColumn ? (
        <TableCell className="overflow-hidden text-center">
          <CostCell document={doc} size="sm" />
        </TableCell>
      ) : null}

      {/* Created Date */}
      <TableCell className="max-w-0 overflow-hidden text-muted-foreground">
        {doc.created_at ? (
          <span
            className="block truncate whitespace-nowrap"
            title={new Date(doc.created_at).toLocaleString()}
          >
            {formatDistanceToNow(new Date(doc.created_at), { addSuffix: true })}
          </span>
        ) : (
          '-'
        )}
      </TableCell>

      {/* Last Updated Date — shows when doc was last reprocessed/rebuilt */}
      <TableCell className="max-w-0 overflow-hidden text-muted-foreground">
        {(doc.updated_at || doc.processed_at) ? (
          <span
            className="block truncate whitespace-nowrap"
            title={new Date(doc.updated_at ?? doc.processed_at!).toLocaleString()}
          >
            {formatDistanceToNow(new Date(doc.updated_at ?? doc.processed_at!), { addSuffix: true })}
          </span>
        ) : (
          '-'
        )}
      </TableCell>

      {/* Actions */}
      <TableCell className="overflow-hidden" onClick={(e) => e.stopPropagation()}>
        <QuickActionButtons
          doc={doc}
          onViewDetails={onViewDetails}
          onPreview={onClick}
          onViewInGraph={onViewInGraph}
          onRetry={onRetry}
          onRetryLabels={onRetryLabels}
          isRetrying={isRetrying}
        >
          <DocumentActionsMenu
            doc={doc}
            onViewPdf={onViewPdf}
            onCancel={onCancel}
            onReprocess={onReprocess}
            onDelete={onDelete}
            isCancelling={isCancelling}
            isDeleting={isDeleting}
          />
        </QuickActionButtons>
      </TableCell>
    </TableRow>
  );
});

export default DocumentTableRow;
