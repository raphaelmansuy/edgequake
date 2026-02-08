/**
 * DocumentTableStates - Loading skeleton and empty state for document table
 *
 * @fileoverview Extracted from DocumentManager (OODA-12)
 * WHY: SRP - Table state displays are distinct from data rendering
 *
 * @module edgequake_webui/components/documents/document-table-states
 */
'use client';

import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { FileText, Upload } from 'lucide-react';

export interface DocumentTableStatesProps {
  /** Whether data is currently loading */
  isLoading: boolean;
  /** Whether document list is empty (after loading) */
  isEmpty: boolean;
  /** Callback when upload button is clicked */
  onUploadClick: () => void;
  /** Number of skeleton rows to show (default: 5) */
  rowCount?: number;
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
 * Empty state with upload CTA
 */
function EmptyState({ onUploadClick }: { onUploadClick: () => void }) {
  return (
    <div className="text-center py-16 text-muted-foreground border rounded-lg bg-muted/5">
      <FileText className="h-12 w-12 mx-auto mb-4 opacity-40" />
      <p className="font-medium text-lg text-foreground">No documents yet</p>
      <p className="text-sm mt-2 max-w-sm mx-auto">
        Drag & drop files above or click to upload. Build your knowledge graph
        from documents.
      </p>
      <Button variant="outline" className="mt-4" onClick={onUploadClick}>
        <Upload className="h-4 w-4 mr-2" />
        Upload Documents
      </Button>
    </div>
  );
}

/**
 * DocumentTableStates - Conditional states for document table
 *
 * Returns:
 * - Loading skeleton when isLoading
 * - Empty state when isEmpty and not loading
 * - null when neither (table should render)
 */
export function DocumentTableStates({
  isLoading,
  isEmpty,
  onUploadClick,
  rowCount = 5,
}: DocumentTableStatesProps) {
  if (isLoading) {
    return <LoadingSkeleton rowCount={rowCount} />;
  }

  if (isEmpty) {
    return <EmptyState onUploadClick={onUploadClick} />;
  }

  return null;
}

export default DocumentTableStates;
