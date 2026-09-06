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
import { useUsers } from '@/hooks/use-users';
import { principalDisplayName } from '@/components/security/principal-select';
import {
  documentTableCols,
  INVENTORY_CARDS_CLASS,
  INVENTORY_TABLE_CLASS,
} from '@/lib/documents/document-table-columns';
import type { SortDirection, SortField } from '@/lib/documents/document-sort';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { Document } from '@/types';
import { FileText } from 'lucide-react';
import { memo, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { DocumentTableRow } from './document-table-row';
import { DocumentInventoryCard } from './document-inventory-card';
import { DocumentTableStates } from './document-table-states';
import { SortableColumnHeader } from './sortable-column-header';

/** Estimated row height for the virtualizer (px). */
const ESTIMATED_ROW_HEIGHT = 52;

/**
 * Shared colgroup — percentage widths so Title never collapses under
 * table-fixed when the inventory pane is narrow (preview panel open).
 */
function TableColGroup({
  showCostColumn,
  showAbacColumns,
}: {
  showCostColumn: boolean;
  showAbacColumns: boolean;
}) {
  const cols = documentTableCols(showCostColumn, showAbacColumns);
  return (
    <colgroup>
      <col style={{ width: cols.checkbox }} />
      <col style={{ width: cols.title }} />
      <col style={{ width: cols.status }} />
      {'class' in cols ? <col style={{ width: cols.class }} /> : null}
      {'share' in cols ? <col style={{ width: cols.share }} /> : null}
      {'owner' in cols ? <col style={{ width: cols.owner }} /> : null}
      <col style={{ width: cols.entities }} />
      {'cost' in cols ? <col style={{ width: cols.cost }} /> : null}
      <col style={{ width: cols.created }} />
      <col style={{ width: cols.updated }} />
      <col style={{ width: cols.actions }} />
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
  /** Document IDs currently in an active ingestion run (SPEC-048 mute others) */
  activeRunDocumentIds?: Set<string>;
  onSelectAll: (checked: boolean) => void;
  onSelectOne: (id: string, checked: boolean) => void;
  onRowClick: (doc: Document) => void;
  onRowDoubleClick: (doc: Document) => void;
  onViewDetails: (doc: Document) => void;
  onViewInGraph: (doc: Document) => void;
  onViewPdf: (doc: Document) => void;
  onRetry: (id: string) => void;
  onRetryLabels?: (doc: Document) => void;
  onReprocess: (id: string) => void;
  onCancel: (trackId: string) => void;
  onDelete: (id: string) => void;
  isRetrying: boolean;
  isCancelling: boolean;
  /** SPEC-050: IDs of documents currently being deleted (for row dimming). */
  deletingDocumentIds?: Set<string>;
  onUploadClick: () => void;
  onClearFilter?: () => void;
  /** True when ingest/upload is busy but the list is still empty */
  isBusyUpdating?: boolean;
  /** Active sort field (shared with toolbar — DRY) */
  sortField: SortField;
  /** Active sort direction */
  sortDirection: SortDirection;
  /** Column header sort toggle */
  onSort: (field: SortField) => void;
  /** SPEC-099: Cost column opt-in */
  showCostColumn?: boolean;
  /** SPEC-146: Class / Share columns when DOC_ABAC is on */
  showAbacColumns?: boolean;
  /** SPEC-099: overflow honesty affordance */
  overflowLabel?: string | null;
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
  activeRunDocumentIds,
  onSelectAll,
  onSelectOne,
  onRowClick,
  onRowDoubleClick,
  onViewDetails,
  onViewInGraph,
  onViewPdf,
  onRetry,
  onRetryLabels,
  onReprocess,
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
  deletingDocumentIds,
  onUploadClick,
  onClearFilter,
  isBusyUpdating = false,
  sortField,
  sortDirection,
  onSort,
  showCostColumn = false,
  showAbacColumns = false,
  overflowLabel = null,
}: DocumentTableSectionProps) {
  const { t } = useTranslation();
  const { users } = useUsers();
  const ownerNames = Object.fromEntries(
    users.map((u) => [u.user_id, principalDisplayName(u, u.user_id)]),
  );
  const scrollRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: documents.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ESTIMATED_ROW_HEIGHT,
    overscan: 8,
  });

  const virtualItems = virtualizer.getVirtualItems();
  const totalVirtualHeight = virtualizer.getTotalSize();
  // Offset for the visible window inside the spacer (TanStack absolute pattern).
  // Do NOT use <tr height=padding> spacers — table min-height leaks into
  // document.scrollHeight and creates the page-level white-band scrollbar.
  const windowOffset = virtualItems[0]?.start ?? 0;

  const showTable = !isLoading && documents.length > 0;

  // SPEC-099: must be a real flex child (not a Fragment). A Fragment breaks the
  // min-h-0 / flex-1 chain → virtualizer padding blows page height, header +
  // dropzone scroll away, and a large empty white band appears below rows.
  return (
    <div
      className="flex min-h-0 min-w-0 flex-1 flex-col overflow-clip"
      data-testid="documents-inventory-section"
    >
      {/* ── ZONE 1: shrink-0 header (never inside the scroll container) ── */}
      <div className="shrink-0 px-4 pt-3 bg-background">
        {/* Count / filter info */}
        {showTable && (
          <div className="flex items-center gap-2 mb-1.5">
            <FileText className="h-3.5 w-3.5 text-muted-foreground" aria-hidden="true" />
            <span
              className="text-xs text-muted-foreground tabular-nums"
              data-testid="spec099-inventory-count"
            >
              {searchQuery || statusFilter !== 'all'
                ? t('documents.filter.showingFiltered', '{{count}} of {{total}}', {
                    count: documents.length,
                    total: totalCount,
                  })
                : t('documents.documentCount', '{{count}} documents', { count: totalCount })}
            </span>
            {overflowLabel ? (
              <span
                className="text-xs text-amber-700 dark:text-amber-400"
                data-testid="spec099-scale-overflow"
              >
                {overflowLabel}
              </span>
            ) : null}
          </div>
        )}

        {/* Column header row — physically outside the scroll container (md+) */}
        {showTable && (
          <div className={`${INVENTORY_TABLE_CLASS} border border-border border-b-0 rounded-t-lg bg-muted/40 overflow-hidden shadow-sm`}>
            <table className="w-full table-fixed caption-bottom text-sm" role="presentation">
              <TableColGroup
                showCostColumn={showCostColumn}
                showAbacColumns={showAbacColumns}
              />
              <TableHeader>
                <TableRow className="hover:bg-transparent">
                  <TableHead scope="col" className="rounded-tl-lg w-11 overflow-hidden">
                    <Checkbox
                      checked={isAllSelected}
                      onCheckedChange={(checked) => onSelectAll(!!checked)}
                      aria-label={t('documents.bulk.selectAll', 'Select all')}
                    />
                  </TableHead>
                  <SortableColumnHeader
                    field="title"
                    label={t('documents.table.title', 'Title')}
                    activeField={sortField}
                    direction={sortDirection}
                    onSort={onSort}
                    className="max-w-0 overflow-hidden"
                  />
                  <SortableColumnHeader
                    field="status"
                    label={t('documents.table.status', 'Status')}
                    activeField={sortField}
                    direction={sortDirection}
                    onSort={onSort}
                    className="overflow-hidden"
                  />
                  {showAbacColumns ? (
                    <>
                      <TableHead
                        scope="col"
                        className="overflow-hidden text-xs"
                        data-testid="spec146-col-class"
                      >
                        {t('documents.table.classification', 'Class')}
                      </TableHead>
                      <TableHead
                        scope="col"
                        className="overflow-hidden text-xs"
                        data-testid="spec146-col-share"
                      >
                        {t('documents.table.shareMode', 'Share')}
                      </TableHead>
                      <TableHead
                        scope="col"
                        className="overflow-hidden text-xs"
                        data-testid="spec146-col-owner"
                      >
                        {t('documents.table.owner', 'Owner')}
                      </TableHead>
                    </>
                  ) : null}
                  <SortableColumnHeader
                    field="entity_count"
                    label={t('documents.table.entities', 'Entities')}
                    activeField={sortField}
                    direction={sortDirection}
                    onSort={onSort}
                    align="center"
                    className="overflow-hidden"
                  />
                  {showCostColumn ? (
                    <SortableColumnHeader
                      field="cost_usd"
                      label={t('documents.table.cost', 'Cost')}
                      activeField={sortField}
                      direction={sortDirection}
                      onSort={onSort}
                      align="center"
                      className="overflow-hidden"
                    />
                  ) : null}
                  <SortableColumnHeader
                    field="created_at"
                    label={t('documents.table.created', 'Created')}
                    activeField={sortField}
                    direction={sortDirection}
                    onSort={onSort}
                    className="overflow-hidden"
                  />
                  <SortableColumnHeader
                    field="updated_at"
                    label={t('documents.table.updated', 'Last Updated')}
                    activeField={sortField}
                    direction={sortDirection}
                    onSort={onSort}
                    className="overflow-hidden"
                  />
                  <TableHead scope="col" className="rounded-tr-lg overflow-hidden">
                    <span className="sr-only">{t('documents.table.actions', 'Actions')}</span>
                  </TableHead>
                </TableRow>
              </TableHeader>
            </table>
          </div>
        )}
      </div>

      {/* ── ZONE 2: flex-1 scroll container (body only) ── */}
      <div
        ref={scrollRef}
        className="min-h-[9rem] flex-1 overflow-x-hidden overflow-y-auto overscroll-contain px-4 pb-3 [contain:paint]"
        data-testid="documents-table-scroll"
      >
        {/* Loading / empty states — shown when no table */}
        <DocumentTableStates
          isLoading={isLoading}
          isEmpty={documents.length === 0}
          onUploadClick={onUploadClick}
          statusFilter={statusFilter}
          searchQuery={searchQuery}
          onClearFilter={onClearFilter}
          isBusyUpdating={isBusyUpdating}
        />

        {showTable && (
          <>
            {/* Mobile / tablet cards (< md) */}
            <div
              className={INVENTORY_CARDS_CLASS}
              data-testid="spec146-doc-cards"
            >
              {documents.map((doc) => {
                const bareId = doc.id.replace(/^staging:/, '');
                const isLiveRun =
                  Boolean(activeRunDocumentIds?.has(doc.id)) ||
                  Boolean(activeRunDocumentIds?.has(bareId));
                void isLiveRun;
                return (
                  <DocumentInventoryCard
                    key={doc.id}
                    doc={doc}
                    isSelected={selectedIds.has(doc.id)}
                    isActive={selectedDocument?.id === doc.id}
                    showAbacColumns={showAbacColumns}
                    ownerNames={ownerNames}
                    onSelect={onSelectOne}
                    onClick={onRowClick}
                    onViewDetails={onViewDetails}
                    onViewInGraph={onViewInGraph}
                    onViewPdf={onViewPdf}
                    onRetry={onRetry}
                    onRetryLabels={onRetryLabels}
                    onReprocess={onReprocess}
                    onCancel={onCancel}
                    onDelete={onDelete}
                    isRetrying={isRetrying}
                    isCancelling={isCancelling}
                  />
                );
              })}
            </div>

            {/* Desktop virtualized table (md+) */}
            <div
              className={`relative w-full ${INVENTORY_TABLE_CLASS}`}
              style={{ height: totalVirtualHeight }}
              data-testid="documents-virtual-spacer"
            >
            <div
              className="absolute left-0 right-0 border border-border rounded-b-lg overflow-hidden shadow-sm bg-background"
              style={{ transform: `translateY(${windowOffset}px)` }}
              aria-label={t('documents.table.ariaLabel', 'Documents list')}
            >
              <table className="w-full table-fixed caption-bottom text-sm">
                <TableColGroup
                  showCostColumn={showCostColumn}
                  showAbacColumns={showAbacColumns}
                />
                <TableBody>
                  {virtualItems.map((virtualRow) => {
                    const doc = documents[virtualRow.index];
                    if (!doc) return null;
                    const bareId = doc.id.replace(/^staging:/, "");
                    const isLiveRun =
                      Boolean(activeRunDocumentIds?.has(doc.id)) ||
                      Boolean(activeRunDocumentIds?.has(bareId));
                    const isBackground =
                      Boolean(activeRunDocumentIds && activeRunDocumentIds.size > 0) &&
                      !isLiveRun;
                    return (
                      <DocumentTableRow
                        key={doc.id}
                        doc={doc}
                        index={virtualRow.index}
                        isSelected={selectedIds.has(doc.id)}
                        isActive={selectedDocument?.id === doc.id}
                        isBackground={isBackground}
                        isLiveRun={isLiveRun}
                        showCostColumn={showCostColumn}
                        showAbacColumns={showAbacColumns}
                        searchQuery={searchQuery}
                        onSelect={onSelectOne}
                        onClick={onRowClick}
                        onDoubleClick={onRowDoubleClick}
                        onViewDetails={onViewDetails}
                        onViewInGraph={onViewInGraph}
                        onViewPdf={onViewPdf}
                        onRetry={onRetry}
                        onRetryLabels={onRetryLabels}
                        ownerNames={ownerNames}
                        onReprocess={onReprocess}
                        onCancel={onCancel}
                        onDelete={onDelete}
                        isRetrying={isRetrying}
                        isCancelling={isCancelling}
                        isDeleting={
                          (deletingDocumentIds?.has(doc.id) ?? false) ||
                          (doc.status || '').toLowerCase() === 'deleting' ||
                          (doc.current_stage || '').toLowerCase() === 'deleting'
                        }
                      />
                    );
                  })}
                </TableBody>
              </table>
            </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
});

export default DocumentTableSection;
