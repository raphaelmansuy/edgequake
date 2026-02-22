/**
 * @module DocumentTableSection
 * @description Document table with header, states, rows, and pagination.
 * Extracted from DocumentManager for SRP compliance (OODA-26).
 * 
 * WHY: Table JSX was inline in DocumentManager causing bloat.
 * This component shows:
 * - Table header with document count
 * - Loading skeleton and empty states
 * - Document rows with selection and actions
 * - Pagination controls
 * 
 * @implements FEAT0001 - Document list display
 * @implements FEAT0401 - Document filtering and pagination
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
import type { Document } from '@/types';
import { FileText } from 'lucide-react';
import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { DocumentTableRow } from './document-table-row';
import { DocumentTableStates } from './document-table-states';
import { PaginationControls } from './pagination-controls';

/**
 * Props for DocumentTableSection component.
 */
export interface DocumentTableSectionProps {
  /** Documents to display */
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
  /** Handler for select all checkbox */
  onSelectAll: (checked: boolean) => void;
  /** Handler for individual selection */
  onSelectOne: (id: string, checked: boolean) => void;
  /** Handler for row click (preview) */
  onRowClick: (doc: Document) => void;
  /** Handler for row double-click (navigate) */
  onRowDoubleClick: (doc: Document) => void;
  /** Handler for view details action */
  onViewDetails: (doc: Document) => void;
  /** Handler for view in graph action */
  onViewInGraph: (doc: Document) => void;
  /** Handler for view PDF action */
  onViewPdf: (doc: Document) => void;
  /** Handler for retry action */
  onRetry: (id: string) => void;
  /** Handler for cancel action */
  onCancel: (trackId: string) => void;
  /** Handler for delete action */
  onDelete: (id: string) => void;
  /** Whether retrying is in progress */
  isRetrying: boolean;
  /** Whether cancelling is in progress */
  isCancelling: boolean;
  /** Handler for upload button click */
  onUploadClick: () => void;
  /** Current page */
  currentPage: number;
  /** Total pages */
  totalPages: number;
  /** Page size */
  pageSize: number;
  /** Handler for page change */
  onPageChange: (page: number) => void;
  /** Handler for page size change */
  onPageSizeChange: (size: number) => void;
  /** Optional callback to clear active filters/search (shows clear button in empty state) */
  onClearFilter?: () => void;
}

/**
 * Document table section with loading states and pagination.
 * WHY: Wrapped in memo to prevent re-renders when DocumentManager state
 * unrelated to the table (e.g., preview panel, dialog state) changes.
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
  onCancel,
  onDelete,
  isRetrying,
  isCancelling,
  onUploadClick,
  currentPage,
  totalPages,
  pageSize,
  onPageChange,
  onPageSizeChange,
  onClearFilter,
}: DocumentTableSectionProps) {
  const { t } = useTranslation();

  return (
    <>
      {/* Scrollable Documents Table Zone */}
      <div className="flex-1 min-h-0 overflow-auto">
        <div className="px-4 py-3">
          {/* Table Header */}
          <div className="flex items-center gap-2 mb-3">
            <FileText className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">
              {t('documents.documentCount', 'Documents ({{count}})', { count: documents.length })}
            </span>
          </div>
          
          {/* OODA-12: Loading skeleton and empty state */}
          <DocumentTableStates
            isLoading={isLoading}
            isEmpty={documents.length === 0}
            onUploadClick={onUploadClick}
            statusFilter={statusFilter}
            searchQuery={searchQuery}
            onClearFilter={onClearFilter}
          />
          
          {!isLoading && documents.length > 0 && (
            <div className="border rounded-lg overflow-hidden shadow-sm">
              <Table aria-label="Documents list">
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
                    <TableHead scope="col" className="w-25"><span className="sr-only">{t('documents.table.actions', 'Actions')}</span></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {documents.map((doc, index) => (
                    <DocumentTableRow
                      key={doc.id}
                      doc={doc}
                      index={index}
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
                      onCancel={onCancel}
                      onDelete={onDelete}
                      isRetrying={isRetrying}
                      isCancelling={isCancelling}
                    />
                  ))}
                </TableBody>
              </Table>
            </div>
          )}
        </div>
      </div>
          
      {/* Fixed Pagination Footer */}
      {documents.length > 0 && (
        <div className="shrink-0 px-4 py-3 border-t bg-background">
          {/* Show filtered vs total count when filtering */}
          {(searchQuery || statusFilter !== 'all') && (
            <p className="text-xs text-muted-foreground mb-2 text-center">
              {t('documents.filter.showing', 'Showing {{count}} of {{total}} documents', {
                count: documents.length,
                total: totalCount,
              })}
              {searchQuery && ` ${t('documents.filter.matching', 'matching "{{query}}"', { query: searchQuery })}`}
            </p>
          )}
          <PaginationControls
            currentPage={currentPage}
            totalPages={totalPages}
            pageSize={pageSize}
            onPageChange={onPageChange}
            onPageSizeChange={(newSize) => {
              onPageSizeChange(newSize);
              onPageChange(1);
            }}
          />
        </div>
      )}
    </>
  );
});

export default DocumentTableSection;
