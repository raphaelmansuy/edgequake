/**
 * @module DocumentTableSection
 * @description Document table with virtual scrolling, states, and rows.
 * Extracted from DocumentManager for SRP compliance (OODA-26).
 *
 * VS-01: Uses @tanstack/react-virtual (spacer-row pattern) to render only
 * visible rows — works with native <table> layout so column widths stay
 * aligned with the sticky header.
 *
 * @implements FEAT0001 - Document list display
 * @implements FEAT0401 - Document filtering
 */
'use client';

import { Checkbox } from '@/components/ui/checkbox';
import {
    Table,
    TableBody,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { Document } from '@/types';
import { FileText } from 'lucide-react';
import { memo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { DocumentTableRow } from './document-table-row';
import { DocumentTableStates } from './document-table-states';

/** Estimated row height used by the virtualizer (px). */
const ESTIMATED_ROW_HEIGHT = 52;

/**
 * Props for DocumentTableSection component.
 */
export interface DocumentTableSectionProps {
  /** Documents to display (all, not paginated — virtualizer handles windowing) */
  documents: Document[];
  /** Total count for filtering info */
  totalCount: number;
  /** Whether data is loading */
  isLoading: boolean;
  /** Selected document IDs */
  selectedIds: Set<string>;
  /** Currently active document for preview */
  selectedDocument: Document | null;
  /** Current search query */
  searchQuery: string;
  /** Current status filter */
  statusFilter: string;
  /** Whether all are selected */
  isAllSelected: boolean;
  onSelectAll: (checked: boolean) => void;
  onSelectOne: (id: string, checked: boolean) => void;
  onRowClick: (doc: Document) => void;
  onRowDoubleClick: (doc: Document) => void;
  onViewDetails: (doc: Document) => void;
  onViewInGraph: (doc: Document) => void;
  onViewPdf: (doc: Document) => void;
  onRetry: (id: string) => void;
  onReprocess: (id: string) => void;
  onCancel: (trackId: string) => void;
  onDelete: (id: string) => void;
  isRetrying: boolean;
  isCancelling: boolean;
  onUploadClick: () => void;
  onClearFilter?: () => void;
}

/**
 * Document table with virtual scrolling.
 * VS-01: Spacer-row virtualizer pattern — header stays sticky, columns align.
 * WHY: Wrapped in memo so re-renders from preview-panel/dialog state changes
 * in DocumentManager don't cause the entire table to re-render.
 */
export const DocumentTableSection = memo(function DocumentTableSection({
  documents,
  totalCount,
  isLoading,
  selectedIds,
  selectedDocument,
  searchQuery,
  statusFilter,
  isAllSelected,
  onSelectAll,
  onSelectOne,
  onRowClick,
  onRowDoubleClick,
  onViewDetails,
  onViewInGraph,
  onViewPdf,
  onRetry,
  onReprocess,
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
  onUploadClick,
  onClearFilter,
}: DocumentTableSectionProps) {
  const { t } = useTranslation();

  // Scroll container ref — virtualizer needs the actual scroll element
  const scrollRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: documents.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_ROW_HEIGHT,
    overscan: 8, // extra rows above/below viewport for smooth scrolling
  });

  const virtualItems = virtualizer.getVirtualItems();
  const totalVirtualHeight = virtualizer.getTotalSize();

  // Padding rows keep total scroll height correct while only visible rows render
  const paddingTop = virtualItems.length > 0 ? virtualItems[0].start : 0;
  const paddingBottom =
    virtualItems.length > 0
      ? totalVirtualHeight - virtualItems[virtualItems.length - 1].end
      : 0;

  return (
    /*
     * WHY: The outer div is a flex child (flex-1 min-h-0 overflow-hidden) that
     * establishes a bounded containing block. The inner scroll div uses
     * absolute inset-0 + overflow-auto which GUARANTEES a scroll container
     * regardless of what `h-full` or percentage heights do in the flex chain.
     * The sticky <thead> sticks to this absolute div's scroll context.
     */
    <div className="flex-1 min-h-0 overflow-hidden relative">
      <div ref={scrollRef} className="absolute inset-0 overflow-auto">
      <div className="px-4 pt-3 pb-2">
        {/* Count header */}
        {!isLoading && documents.length > 0 && (
          <div className="flex items-center gap-2 mb-2">
            <FileText className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
            <span className="text-xs text-muted-foreground tabular-nums">
              {searchQuery || statusFilter !== 'all'
                ? t('documents.filter.showingFiltered', '{{count}} of {{total}}', {
                    count: documents.length,
                    total: totalCount,
                  })
                : t('documents.documentCount', '{{count}} documents', { count: totalCount })}
            </span>
          </div>
        )}

        {/* Loading / empty states */}
        <DocumentTableStates
          isLoading={isLoading}
          isEmpty={documents.length === 0}
          onUploadClick={onUploadClick}
          statusFilter={statusFilter}
          searchQuery={searchQuery}
          onClearFilter={onClearFilter}
        />

        {!isLoading && documents.length > 0 && (
          /* WHY: [overflow:clip] rounds corners without creating a BFC.
             overflow-hidden would trap the sticky <thead> inside the div's
             scroll context, preventing it from sticking to the outer
             scrollRef container. overflow:clip clips visually but does
             NOT create a scroll container, so sticky top-0 on <thead>
             correctly reaches scrollRef. */
          <div className="border rounded-lg shadow-sm [overflow:clip]">
            <Table aria-label={t('documents.table.ariaLabel', 'Documents list')}>
              <TableHeader className="bg-muted/50 sticky top-0 z-10">
                <TableRow className="hover:bg-transparent">
                  <TableHead scope="col" className="w-10">
                    <Checkbox
                      checked={isAllSelected}
                      onCheckedChange={(checked) => onSelectAll(!!checked)}
                      aria-label={t('documents.bulk.selectAll', 'Select all')}
                    />
                  </TableHead>
                  <TableHead scope="col">{t('documents.table.title', 'Title')}</TableHead>
                  <TableHead scope="col">{t('documents.table.status', 'Status')}</TableHead>
                  <TableHead scope="col" className="text-center">{t('documents.table.entities', 'Entities')}</TableHead>
                  <TableHead scope="col" className="text-center">{t('documents.table.cost', 'Cost')}</TableHead>
                  <TableHead scope="col">{t('documents.table.created', 'Created')}</TableHead>
                  <TableHead scope="col">{t('documents.table.updated', 'Last Updated')}</TableHead>
                  <TableHead scope="col" className="w-25">
                    <span className="sr-only">{t('documents.table.actions', 'Actions')}</span>
                  </TableHead>
                </TableRow>
              </TableHeader>

              <TableBody>
                {/* Top spacer row — maintains total scroll height above visible items */}
                {paddingTop > 0 && (
                  <tr aria-hidden="true">
                    <td style={{ height: paddingTop }} />
                  </tr>
                )}

                {/* Visible rows only */}
                {virtualItems.map((virtualRow) => {
                  const doc = documents[virtualRow.index];
                  if (!doc) return null;
                  return (
                    <DocumentTableRow
                      key={doc.id}
                      doc={doc}
                      index={virtualRow.index}
                      isSelected={selectedIds.has(doc.id)}
                      isActive={selectedDocument?.id === doc.id}
                      searchQuery={searchQuery}
                      onSelect={onSelectOne}
                      onClick={onRowClick}
                      onDoubleClick={onRowDoubleClick}
                      onViewDetails={onViewDetails}
                      onViewInGraph={onViewInGraph}
                      onViewPdf={onViewPdf}
                      onRetry={onRetry}
                      onReprocess={onReprocess}
                      onCancel={onCancel}
                      onDelete={onDelete}
                      isRetrying={isRetrying}
                      isCancelling={isCancelling}
                    />
                  );
                })}

                {/* Bottom spacer row — maintains scroll height below visible items */}
                {paddingBottom > 0 && (
                  <tr aria-hidden="true">
                    <td style={{ height: paddingBottom }} />
                  </tr>
                )}
              </TableBody>
            </Table>
          </div>
        )}
      </div>
      </div>
    </div>
  );
});

export default DocumentTableSection;
