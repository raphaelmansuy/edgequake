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
  /** Called when Retry action is triggered */
  onRetry: (docId: string) => void;
  /** Called when Cancel action is triggered */
  onCancel: (trackId: string) => void;
  /** Called when Delete action is triggered */
  onDelete: (docId: string) => void;
  /** Whether a retry operation is pending */
  isRetrying: boolean;
  /** Whether a cancel operation is pending */
  isCancelling: boolean;
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
  searchQuery,
  onSelect,
  onClick,
  onDoubleClick,
  onViewDetails,
  onViewInGraph,
  onViewPdf,
  onRetry,
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
}: DocumentTableRowProps) {
  const { t } = useTranslation();

  // WHY: Visual distinction for document status
  const rowClassName = cn(
    'cursor-pointer transition-colors duration-150',
    'hover:bg-primary/5 dark:hover:bg-primary/10',
    isActive && 'bg-primary/10 dark:bg-primary/15 ring-1 ring-primary/20',
    index % 2 === 0 ? 'bg-background' : 'bg-muted/20',
    // OODA-25: Failed/cancelled documents highlight
    doc.status === 'failed' &&
      'bg-red-50/50 dark:bg-red-950/20 border-l-4 border-l-red-500',
    doc.status === 'partial_failure' &&
      'bg-orange-50/50 dark:bg-orange-950/20 border-l-4 border-l-orange-500',
    doc.status === 'cancelled' &&
      'bg-gray-50/50 dark:bg-gray-950/20 border-l-4 border-l-gray-400'
  );

  const { icon: FileIcon, color } = getFileTypeIcon(doc.file_name);
  const displayTitle =
    doc.title || doc.file_name || `Document ${doc.id.slice(0, 8)}`;

  // OODA-34: "New" indicator for documents created within 1 hour
  const isNew =
    doc.created_at &&
    new Date().getTime() - new Date(doc.created_at).getTime() < 3600000;

  return (
    <TableRow
      className={rowClassName}
      onClick={() => onClick(doc)}
      onDoubleClick={() => onDoubleClick(doc)}
    >
      {/* Selection Checkbox */}
      <TableCell onClick={(e) => e.stopPropagation()}>
        <Checkbox
          checked={isSelected}
          onCheckedChange={(checked) => onSelect(doc.id, !!checked)}
          aria-label={t('documents.bulk.select', 'Select')}
        />
      </TableCell>

      {/* Title with File Type Icon */}
      <TableCell className="font-medium">
        <div className="flex flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <FileIcon className={cn('h-4 w-4 shrink-0', color)} />
            <span className="truncate">
              {highlightMatches(displayTitle, searchQuery)}
            </span>
          </div>
          {/* Error message for failed/cancelled documents */}
          {(doc.status === 'failed' || doc.status === 'partial_failure' || doc.status === 'cancelled') &&
            doc.error_message && (
              <ErrorMessagePopover
                message={doc.error_message}
                documentId={doc.id}
                onRetry={() => onRetry(doc.id)}
                isRetrying={isRetrying}
              />
            )}
          {/* Cancelled indicator when no error message */}
          {doc.status === 'cancelled' && !doc.error_message && (
            <span className="text-xs text-muted-foreground">
              {t('documents.cancelled.subtitle', 'Processing was cancelled')}
            </span>
          )}
        </div>
      </TableCell>

      {/* Status Badge */}
      <TableCell>
        <div className="flex flex-col gap-1">
          <EnhancedStatusBadge document={doc} />
          {/* Show stage_message for PDF conversion progress */}
          {doc.stage_message && doc.current_stage === 'converting' && (
            <span className="text-xs text-muted-foreground truncate">
              {doc.stage_message}
            </span>
          )}
        </div>
      </TableCell>

      {/* Entity Count */}
      <TableCell className="text-center">
        {doc.entity_count ?? doc.chunk_count ?? '-'}
      </TableCell>

      {/* Cost */}
      <TableCell className="text-center">
        <CostCell document={doc} size="sm" />
      </TableCell>

      {/* Created Date */}
      <TableCell className="text-muted-foreground">
        {doc.created_at ? (
          <div className="flex items-center gap-1.5">
            {isNew && (
              <span className="text-xs font-medium text-green-600 dark:text-green-400 animate-pulse">
                NEW
              </span>
            )}
            <span>
              {formatDistanceToNow(new Date(doc.created_at), { addSuffix: true })}
            </span>
          </div>
        ) : (
          '-'
        )}
      </TableCell>

      {/* Last Updated Date — shows when doc was last reprocessed/rebuilt */}
      <TableCell className="text-muted-foreground">
        {(doc.updated_at || doc.processed_at) ? (
          <span title={new Date(doc.updated_at ?? doc.processed_at!).toLocaleString()}>
            {formatDistanceToNow(new Date(doc.updated_at ?? doc.processed_at!), { addSuffix: true })}
          </span>
        ) : (
          '-'
        )}
      </TableCell>

      {/* Actions */}
      <TableCell onClick={(e) => e.stopPropagation()}>
        <QuickActionButtons
          doc={doc}
          onViewDetails={onViewDetails}
          onPreview={onClick}
          onViewInGraph={onViewInGraph}
          onRetry={onRetry}
          isRetrying={isRetrying}
        >
          <DocumentActionsMenu
            doc={doc}
            onViewPdf={onViewPdf}
            onCancel={onCancel}
            onReprocess={onRetry}
            onDelete={onDelete}
            isCancelling={isCancelling}
          />
        </QuickActionButtons>
      </TableCell>
    </TableRow>
  );
});

export default DocumentTableRow;
