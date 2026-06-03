'use client';

import { LayoutList, LayoutGrid } from 'lucide-react';
import type { StatusCounts } from '@/hooks/use-document-filtering';
import type { Document, PipelineStatus } from '@/types';
import { Button } from '@/components/ui/button';
import { BatchActionsBar } from './batch-actions-bar';
import { DocumentDropzone, type DocumentDropzoneProps } from './document-dropzone';
import type { DocStatus, SortField } from './document-filters';
import { DocumentFilters } from './document-filters';
import { DocumentSearchBar } from './document-search-bar';
import { ProcessingStatusSummary } from './processing-status-summary';
import type { UploadingFile } from './types';
import { UploadProgressList } from './upload-progress-list';

/**
 * OODA-30: Document toolbar section component
 * 
 * WHY: Single Responsibility Principle - isolate toolbar UI from main component.
 * Contains search, filters, status summary, dropzone, batch actions, and upload progress.
 */

export interface DocumentToolbarSectionProps {
  // Search
  searchQuery: string;
  onSearchChange: (value: string) => void;
  
  // Filters
  statusFilter: DocStatus;
  onStatusFilterChange: (value: DocStatus) => void;
  sortField: SortField;
  onSortFieldChange: (value: SortField) => void;
  sortDirection: 'asc' | 'desc';
  onSortDirectionChange: (value: 'asc' | 'desc') => void;
  /** Whether documents are currently grouped by topic */
  groupedView: boolean;
  /** Toggle grouped view on/off */
  onGroupedViewChange: (value: boolean) => void;
  statusCounts: StatusCounts;
  
  // Pipeline status
  pipelineStatus: PipelineStatus | undefined;
  documents: Document[];
  onOpenPipelineDetails: () => void;
  
  // Dropzone
  getRootProps: DocumentDropzoneProps['getRootProps'];
  getInputProps: DocumentDropzoneProps['getInputProps'];
  isDragActive: boolean;
  openFileDialog: () => void;
  pdfParserBackend: 'default' | 'vision' | 'edgeparse';
  onPdfParserBackendChange: (value: 'default' | 'vision' | 'edgeparse') => void;
  
  // Bulk actions
  selectedCount: number;
  onBulkReprocess: () => void;
  onBulkDelete: () => void;
  onClearSelection: () => void;
  
  // Upload progress
  uploadingFiles: UploadingFile[];
  isUploading: boolean;
  onRemoveUpload: (index: number) => void;
  onUploadComplete: (index: number) => void;
  onUploadFailed: (index: number, error: string) => void;
}

export function DocumentToolbarSection({
  searchQuery,
  onSearchChange,
  statusFilter,
  onStatusFilterChange,
  sortField,
  onSortFieldChange,
  sortDirection,
  onSortDirectionChange,
  groupedView,
  onGroupedViewChange,
  statusCounts,
  pipelineStatus,
  documents,
  onOpenPipelineDetails,
  getRootProps,
  getInputProps,
  isDragActive,
  openFileDialog,
  pdfParserBackend,
  onPdfParserBackendChange,
  selectedCount,
  onBulkReprocess,
  onBulkDelete,
  onClearSelection,
  uploadingFiles,
  isUploading,
  onRemoveUpload,
  onUploadComplete,
  onUploadFailed,
}: DocumentToolbarSectionProps) {
  return (
    <>
      {/* Search and Filters */}
      <div className="flex flex-col sm:flex-row sm:items-center gap-3 pb-3 border-b">
        <DocumentSearchBar
          value={searchQuery}
          onChange={onSearchChange}
        />
        <DocumentFilters
          status={statusFilter}
          onStatusChange={onStatusFilterChange}
          sortField={sortField}
          onSortFieldChange={onSortFieldChange}
          sortDirection={sortDirection}
          onSortDirectionChange={onSortDirectionChange}
          statusCounts={statusCounts}
        />
        {/* List / Grouped toggle */}
        <div className="flex items-center border rounded-md overflow-hidden shrink-0">
          <Button
            variant={!groupedView ? 'secondary' : 'ghost'}
            size="sm"
            className="rounded-none h-8 px-2.5 gap-1.5"
            onClick={() => onGroupedViewChange(false)}
            title="List view"
          >
            <LayoutList className="h-3.5 w-3.5" />
            <span className="text-xs hidden sm:inline">List</span>
          </Button>
          <Button
            variant={groupedView ? 'secondary' : 'ghost'}
            size="sm"
            className="rounded-none h-8 px-2.5 gap-1.5 border-l"
            onClick={() => onGroupedViewChange(true)}
            title="Grouped by topic"
          >
            <LayoutGrid className="h-3.5 w-3.5" />
            <span className="text-xs hidden sm:inline">Grouped</span>
          </Button>
        </div>
      </div>

      {/* Processing Status Summary */}
      {pipelineStatus && (
        <ProcessingStatusSummary
          pipelineStatus={pipelineStatus}
          documents={documents}
          onOpenDetails={onOpenPipelineDetails}
        />
      )}

      {/* Compact Upload Zone */}
      <DocumentDropzone
        getRootProps={getRootProps}
        getInputProps={getInputProps}
        isDragActive={isDragActive}
        openFileDialog={openFileDialog}
        pdfParserBackend={pdfParserBackend}
        onPdfParserBackendChange={onPdfParserBackendChange}
      />

      {/* Bulk Actions Bar */}
      <BatchActionsBar
        selectedCount={selectedCount}
        onReprocess={onBulkReprocess}
        onDelete={onBulkDelete}
        onClear={onClearSelection}
      />

      {/* Upload Progress */}
      <UploadProgressList
        uploadingFiles={uploadingFiles}
        isUploading={isUploading}
        onRemove={onRemoveUpload}
        onComplete={onUploadComplete}
        onFailed={onUploadFailed}
      />
    </>
  );
}
