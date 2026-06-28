/**
 * @module DocumentTableSection
 * @description Document table with virtual scrolling and a truly fixed header.
 *
 * Architecture:
 *   ┌─────────────────────────────────────────┐
 *   │  shrink-0  │ Header row (NEVER scrolls) │
 *   ├────────────┤─────────────────────────────┤
 *   │ flex-1     │ Body (overflow-y-auto)       │
 *   │ overflow   │ Virtual rows scroll here     │
 *   └────────────┴─────────────────────────────┘
 *
 * WHY: Any approach that puts the header INSIDE the scroll container
 * (sticky, absolute positioning, overflow:clip tricks) fails because the
 * scroll container IS the parent — it scrolls the header away.
 * The only reliable solution is to place the header in a shrink-0 sibling
 * of the scroll container, outside the scrollable region entirely.
 * Column width parity is maintained via a shared <colgroup> on both tables
 * using table-fixed layout.
 *
 * @implements FEAT0001 - Document list display
 * @implements FEAT0401 - Document filtering
 */
'use client';

import { Checkbox } from '@/components/ui/checkbox';
import {
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

/** Estimated row height for the virtualizer (px). */
const ESTIMATED_ROW_HEIGHT = 52;

/**
 * Shared colgroup — defines column widths for BOTH the header table and
 * the body table so they stay in perfect alignment (DRY / single source of truth).
 * table-fixed on both tables means colgroup widths are authoritative.
 */
function TableColGroup() {
  return (
    <colgroup>
      <col style={{ width: '2.5rem' }} /><col /><col style={{ width: '8.5rem' }} /><col style={{ width: '5rem' }} /><col style={{ width: '5.5rem' }} /><col style={{ width: '9rem' }} /><col style={{ width: '9rem' }} /><col style={{ width: '10.5rem' }} />
    </colgroup>
  );
}

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
  const scrollRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: documents.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_ROW_HEIGHT,
    overscan: 8,
  });

  const virtualItems = virtualizer.getVirtualItems();
  const totalVirtualHeight = virtualizer.getTotalSize();
  const paddingTop = virtualItems.length > 0 ? virtualItems[0].start : 0;
  const paddingBottom =
    virtualItems.length > 0
      ? totalVirtualHeight - virtualItems[virtualItems.length - 1].end
      : 0;

  const showTable = !isLoading && documents.length > 0;

  return (
    <>
      {/* ── ZONE 1: shrink-0 header (never inside the scroll container) ── */}
      <div className="shrink-0 px-4 pt-3 bg-background">
        {/* Count / filter info */}
        {showTable && (
          <div className="flex items-center gap-2 mb-1.5">
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

        {/* Column header row — physically outside the scroll container */}
        {showTable && (
          <div className="border-t border-x rounded-t-lg bg-muted/50 overflow-hidden shadow-sm">
            <table className="w-full table-fixed caption-bottom text-sm" role="presentation">
              <TableColGroup />
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead scope="col" className="rounded-tl-lg">
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
                  <TableHead scope="col" className="rounded-tr-lg">
                    <span className="sr-only">{t('documents.table.actions', 'Actions')}</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
            </table>
          </div>
        )}
      </div>

      {/* ── ZONE 2: flex-1 scroll container (body only) ── */}
      <div ref={scrollRef} className="flex-1 min-h-0 overflow-y-auto px-4 pb-3">
        {/* Loading / empty states — shown when no table */}
        <DocumentTableStates
          isLoading={isLoading}
          isEmpty={documents.length === 0}
          onUploadClick={onUploadClick}
          statusFilter={statusFilter}
          searchQuery={searchQuery}
          onClearFilter={onClearFilter}
        />

        {showTable && (
          <div
            className="border-b border-x rounded-b-lg overflow-hidden shadow-sm"
            aria-label={t('documents.table.ariaLabel', 'Documents list')}
          >
            <table className="w-full table-fixed caption-bottom text-sm">
              <TableColGroup />
              <TableBody>
                {paddingTop > 0 && (
                  <tr aria-hidden="true"><td style={{ height: paddingTop }} /></tr>
                )}
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
                {paddingBottom > 0 && (
                  <tr aria-hidden="true"><td style={{ height: paddingBottom }} /></tr>
                )}
              </TableBody>
            </table>
          </div>
        )}
      </div>
    </>
  );
});

export default DocumentTableSection;
