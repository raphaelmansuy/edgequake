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
import { ZERO_AUTHZ_ANSWER } from '@/lib/query/query-empty-copy';
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
  /**
   * True when uploads/pipeline are busy but the list is still empty
   * (e.g. first PDF ingest before the row appears). Shows an honest
   * "Updating…" empty state instead of "No documents yet".
   */
  isBusyUpdating?: boolean;
}

/**
 * Loading skeleton matching table structure
 * LS-01: aria-busy + aria-label so screen readers announce loading state.
 */
function LoadingSkeleton({ rowCount = 8 }: { rowCount?: number }) {
  // Fill the inventory scrollport so skeleton → rows does not change flex geometry (CLS).
  return (
    <div
      role="status"
      aria-busy="true"
      aria-label="Loading documents..."
      className="h-full min-h-[9rem] border border-border rounded-b-lg overflow-hidden bg-background"
      data-testid="documents-loading-skeleton"
    >
      {[...Array(rowCount)].map((_, i) => (
        <div
          key={i}
          className="flex h-[52px] items-center gap-4 px-4 border-b last:border-b-0 animate-pulse"
          aria-hidden="true"
        >
          <Skeleton className="h-4 w-4 shrink-0 rounded" />
          <Skeleton className="h-4 w-48 shrink-0" />
          <Skeleton className="h-5 w-28 rounded-full shrink-0" />
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
/** Existence-hiding empty browse — same SSOT copy as query ZERO_AUTHZ. */
function ExistenceEmptyState({ onClearFilter }: { onClearFilter?: () => void }) {
  const { t } = useTranslation();
  return (
    <div
      className="text-center py-16 text-muted-foreground border rounded-lg bg-muted/5"
      data-testid="spec146-existence-empty"
    >
      <Search className="h-12 w-12 mx-auto mb-4 opacity-40" />
      <p className="font-medium text-lg text-foreground">
        {t('documents.noFilterResults', ZERO_AUTHZ_ANSWER)}
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
      <Button
        variant="outline"
        className="mt-4"
        onClick={onUploadClick}
        data-testid="documents-browse-files"
      >
        <Upload className="h-4 w-4 mr-2" />
        {t('documents.browseFiles', 'Browse files')}
      </Button>
    </div>
  );
}

/** Honest empty state while list is catching up to an in-flight ingest. */
function BusyUpdatingState() {
  const { t } = useTranslation();
  return (
    <div
      className="text-center py-16 text-muted-foreground border rounded-lg bg-muted/5"
      role="status"
      aria-live="polite"
      data-testid="documents-busy-updating"
    >
      <FileText className="h-12 w-12 mx-auto mb-4 opacity-40 animate-pulse" />
      <p className="font-medium text-lg text-foreground">
        {t('documents.updatingList', 'Updating document list…')}
      </p>
      <p className="text-sm mt-2 max-w-sm mx-auto">
        {t(
          'documents.updatingListSubtitle',
          'Processing is in progress. Documents will appear here shortly.',
        )}
      </p>
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
  isBusyUpdating = false,
}: DocumentTableStatesProps) {
  if (isLoading) {
    return <LoadingSkeleton rowCount={rowCount} />;
  }

  if (isEmpty) {
    // WHY: Only show filter-empty state when a filter/search is actively hiding results.
    const hasActiveFilter = (statusFilter && statusFilter !== 'all') || !!searchQuery;
    if (hasActiveFilter) {
      return <ExistenceEmptyState onClearFilter={onClearFilter} />;
    }
    if (isBusyUpdating) {
      return <BusyUpdatingState />;
    }
    return <EmptyState onUploadClick={onUploadClick} />;
  }

  return null;
}

export default DocumentTableStates;
