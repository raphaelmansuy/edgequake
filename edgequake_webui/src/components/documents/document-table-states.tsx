/**
 * DocumentTableStates - Loading skeleton and empty state for document table
 *
 * @fileoverview Extracted from DocumentManager (OODA-12)
 * WHY: SRP - Table state displays are distinct from data rendering
 * WHY: Filter-aware empty state prevents confusing "no documents" message
 *      when documents exist but are hidden by an active filter/search.
 *
 * @module edgequake_webui/components/documents/document-table-states
 */
'use client';

import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { FileText, Search, Upload } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface DocumentTableStatesProps {
  /** Whether data is currently loading */
  isLoading: boolean;
  /** Whether document list is empty (after loading) */
  isEmpty: boolean;
  /** Callback when upload button is clicked */
  onUploadClick: () => void;
  /** Number of skeleton rows to show (default: 5) */
  rowCount?: number;
  /** Active status filter — used to decide between no-docs vs filter-empty state */
  statusFilter?: string;
  /** Active search query — used to decide between no-docs vs filter-empty state */
  searchQuery?: string;
  /** Callback to clear all active filters/search */
  onClearFilter?: () => void;
}

/**
 * Loading skeleton matching table structure
 */
function LoadingSkeleton({ rowCount = 5 }: { rowCount?: number }) {
  return (
    <div className="border rounded-lg overflow-hidden">
      {[...Array(rowCount)].map((_, i) => (
        <div
          key={i}
          className="flex items-center gap-4 px-4 py-3 border-b last:border-b-0 animate-pulse"
        >
          <Skeleton className="h-4 w-4 shrink-0 rounded" />
          <Skeleton className="h-4 w-48 shrink-0" />
          <Skeleton className="h-5 w-20 rounded-full shrink-0" />
          <Skeleton className="h-4 w-8 shrink-0" />
          <Skeleton className="h-4 w-12 shrink-0" />
          <Skeleton className="h-4 w-24 shrink-0" />
          <Skeleton className="h-6 w-6 rounded-full shrink-0 ml-auto" />
        </div>
      ))}
    </div>
  );
}

/**
 * Empty state shown when a filter/search is active but yields no results.
 * WHY: Distinguishes "no documents in workspace" from "filter hides all docs".
 */
function FilteredEmptyState({ onClearFilter }: { onClearFilter?: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="text-center py-16 text-muted-foreground border rounded-lg bg-muted/5">
      <Search className="h-12 w-12 mx-auto mb-4 opacity-40" />
      <p className="font-medium text-lg text-foreground">
        {t('documents.noFilterResults', 'No matching documents')}
      </p>
      <p className="text-sm mt-2 max-w-sm mx-auto">
        {t(
          'documents.noFilterResultsSubtitle',
          'No documents match the current filter.',
        )}
      </p>
      {onClearFilter && (
        <Button variant="outline" className="mt-4" onClick={onClearFilter}>
          {t('documents.clearFilter', 'Clear filter')}
        </Button>
      )}
    </div>
  );
}

/**
 * Empty state with upload CTA — shown only when no documents exist at all.
 */
function EmptyState({ onUploadClick }: { onUploadClick: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="text-center py-16 text-muted-foreground border rounded-lg bg-muted/5">
      <FileText className="h-12 w-12 mx-auto mb-4 opacity-40" />
      <p className="font-medium text-lg text-foreground">
        {t('documents.noDocuments', 'No documents yet')}
      </p>
      <p className="text-sm mt-2 max-w-sm mx-auto">
        {t(
          'documents.noDocumentsSubtitle',
          'Upload documents to build your knowledge graph',
        )}
      </p>
      <Button variant="outline" className="mt-4" onClick={onUploadClick}>
        <Upload className="h-4 w-4 mr-2" />
        {t('documents.uploadDocuments', 'Upload Documents')}
      </Button>
    </div>
  );
}

/**
 * DocumentTableStates - Conditional states for document table
 *
 * Returns:
 * - Loading skeleton when isLoading
 * - FilteredEmptyState when isEmpty AND a filter/search is active (docs exist but hidden)
 * - EmptyState when isEmpty and no filter is active (workspace has no docs)
 * - null when table data is available (table should render)
 */
export function DocumentTableStates({
  isLoading,
  isEmpty,
  onUploadClick,
  rowCount = 5,
  statusFilter,
  searchQuery,
  onClearFilter,
}: DocumentTableStatesProps) {
  if (isLoading) {
    return <LoadingSkeleton rowCount={rowCount} />;
  }

  if (isEmpty) {
    // WHY: Only show filter-empty state when a filter/search is actively hiding results.
    const hasActiveFilter = (statusFilter && statusFilter !== 'all') || !!searchQuery;
    if (hasActiveFilter) {
      return <FilteredEmptyState onClearFilter={onClearFilter} />;
    }
    return <EmptyState onUploadClick={onUploadClick} />;
  }

  return null;
}

export default DocumentTableStates;
