'use client';

import type { StatusCounts } from '@/hooks/use-document-filtering';
import type { Document, PipelineStatus } from '@/types';
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
  statusCounts: StatusCounts;
  
  // Pipeline status
  pipelineStatus: PipelineStatus | undefined;
  documents: Document[];
  onOpenPipelineDetails: () => void;
  
  // Dropzone
  getRootProps: DocumentDropzoneProps['getRootProps'];
  getInputProps: DocumentDropzoneProps['getInputProps'];
  isDragActive: boolean;
  
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
  statusCounts,
  pipelineStatus,
  documents,
  onOpenPipelineDetails,
  getRootProps,
  getInputProps,
  isDragActive,
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
