/**
 * @module DocumentManager
 * @description Document ingestion and management interface.
 * Supports file upload, progress tracking, status monitoring, and batch operations.
 * 
 * @implements UC0001 - User uploads documents for ingestion
 * @implements UC0007 - User monitors document processing progress
 * @implements UC0008 - User reprocesses failed documents
 * @implements UC0009 - User deletes documents from knowledge graph
 * @implements FEAT0001 - Document ingestion with entity extraction
 * @implements FEAT0003 - Batch document processing
 * @implements FEAT0004 - Processing status tracking per document
 * @implements FEAT0602 - Real-time progress indicators
 * 
 * @enforces BR0302 - Failed documents can be reprocessed
 * @enforces BR0303 - Document deletion cascades to related entities
 * @enforces BR0305 - Cost tracking per document ingestion
 * 
 * @see {@link docs/use_cases.md} UC0001, UC0007-UC0009
 * @see {@link docs/features.md} FEAT0001, FEAT0003
 */
'use client';

import { useTenantStore } from '@/stores/use-tenant-store';
import type { Document } from '@/types';

import { useRouter } from 'next/navigation';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useBulkSelection } from '@/hooks/use-bulk-selection';
import { useDocumentDropzone } from '@/hooks/use-document-dropzone';
import { useDocumentFiltering } from '@/hooks/use-document-filtering';
import { useDocumentHandlers } from '@/hooks/use-document-handlers';
import { useDocumentKeyboard } from '@/hooks/use-document-keyboard';
import { useDocumentMutations } from '@/hooks/use-document-mutations';
import { useDocumentPreferences } from '@/hooks/use-document-preferences';
import { useDocumentQueries } from '@/hooks/use-document-queries';
import { useDocumentTitle } from '@/hooks/use-document-title';
import { useDocumentWebSocket } from '@/hooks/use-document-websocket';
import { useFileUpload } from '@/hooks/use-file-upload';
import { useStuckDetection } from '@/hooks/use-stuck-detection';
import { DocumentErrorAlert } from './document-error-alert';
import { DocumentHeader } from './document-header';
import { DocumentPreviewRightPanel } from './document-preview-right-panel';
import { DocumentTableSection } from './document-table-section';
import { DocumentToolbarSection } from './document-toolbar-section';
import { DuplicateUploadDialog } from './duplicate-upload-dialog';

export function DocumentManager() {
  const { t } = useTranslation();
  const router = useRouter();

  // Get tenant context for query key
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // Selected document for preview panel
  const [selectedDocument, setSelectedDocument] = useState<Document | null>(null);
  const [previewPanelOpen, setPreviewPanelOpen] = useState(false);

  // SPEC-002: Document viewer dialog state for PDF/Markdown side-by-side view
  const [viewerDialogOpen, setViewerDialogOpen] = useState(false);
  const [viewerPdfId, setViewerPdfId] = useState<string | null>(null);

  // Search state
  const [searchQuery, setSearchQuery] = useState('');

  // Pagination state
  const [currentPage, setCurrentPage] = useState(1);

  // OODA-17: Filter, sort, and pagination preferences with localStorage persistence
  const {
    pageSize, setPageSize,
    statusFilter, setStatusFilter,
    sortField, setSortField,
    sortDirection, setSortDirection,
  } = useDocumentPreferences();

  // Pipeline status dialog state
  const [pipelineDialogOpen, setPipelineDialogOpen] = useState(false);

  // OODA-13: Upload state extracted to useFileUpload hook
  const {
    uploadingFiles,
    isUploading,
    handleFilesUpload,
    removeUploadingFile,
    handleUploadComplete,
    handleUploadFailed,
    pendingDuplicates,
    resolvePendingDuplicates,
  } = useFileUpload({
    tenantId: selectedTenantId,
    workspaceId: selectedWorkspaceId,
    onUploadStart: () => setStatusFilter('all'),
  });

  // OODA-14: Document mutations extracted to useDocumentMutations hook
  const {
    deleteMutation,
    deleteAllMutation,
    reprocessMutation,
    cancelMutation,
    retryTaskMutation,
  } = useDocumentMutations({
    onReprocessSuccess: () => setPipelineDialogOpen(true),
  });

  // OODA-29: Document queries extracted to useDocumentQueries hook
  const { data, isLoading, isError, error, refetch, pipelineStatus, queryClient } = useDocumentQueries({
    tenantId: selectedTenantId,
    workspaceId: selectedWorkspaceId,
    currentPage,
    pageSize,
    statusFilter,
  });

  // OODA-05: WebSocket subscription for real-time document status updates
  // WHY: Extracted to useDocumentWebSocket hook for SRP compliance
  useDocumentWebSocket(data?.items, queryClient);

  // OODA-04: Detect stuck documents using extracted hook
  useStuckDetection(data?.items, {
    timeout: 30000,
    checkInterval: 30000,
  });

  // OODA-21: Document dropzone with file validation
  const { getRootProps, getInputProps, isDragActive, openFileDialog } = useDocumentDropzone({
    onFilesAccepted: handleFilesUpload,
    t,
  });

  // OODA-19: Filter and sort documents using extracted hook
  // OODA-20: Also compute status counts in hook
  const { documents, totalCount, totalPages, statusCounts } = useDocumentFiltering({
    documents: data?.items || [],
    searchQuery,
    statusFilter,
    sortField,
    sortDirection,
    pageSize,
    serverStatusCounts: data?.status_counts,
  });

  // OODA-16: Bulk selection extracted to useBulkSelection hook
  const {
    selectedIds,
    selectedCount,
    isAllSelected,
    handleSelectAll,
    handleSelectOne,
    handleClearSelection,
    handleBulkDelete,
    handleBulkReprocess,
  } = useBulkSelection({ documents });

  // OODA-28: Document handlers extracted to useDocumentHandlers hook
  const {
    handleDocumentClick,
    handleDocumentDoubleClick,
    handleViewDetails,
    handlePreviewClose,
    handleViewInGraph,
    handleViewPdf,
  } = useDocumentHandlers({
    setSelectedDocument,
    setPreviewPanelOpen,
    setViewerDialogOpen,
    setViewerPdfId,
  });

  /**
   * OODA-19: Keyboard shortcuts for power users
   * WHY: Keyboard shortcuts improve efficiency and accessibility
   * 
   * Shortcuts:
   * - Escape: Clear selection or close preview panel
   * - Ctrl/Cmd + A: Select all documents
   * - R: Refresh document list (when not in input)
   */
  // OODA-18: Document keyboard shortcuts (Escape, Ctrl+A, R)
  useDocumentKeyboard({
    previewPanelOpen,
    selectedCount,
    onPreviewClose: handlePreviewClose,
    onSelectAll: handleSelectAll,
    onClearSelection: handleClearSelection,
    onRefresh: refetch,
    t,
  });

  // OODA-22: Dynamic page title with document count
  useDocumentTitle({
    totalCount,
    processingCount: pipelineStatus?.running_tasks || 0,
  });

  if (isError) {
    return <DocumentErrorAlert error={error} onRetry={refetch} />;
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Main Content - Flex column for proper scroll zones */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* Fixed Header Zone */}
        <div className="shrink-0 px-4 pt-4 space-y-3 bg-background">
          <DocumentHeader
            totalCount={totalCount}
            failedCount={statusCounts.failed + statusCounts.cancelled}
            pipelineIsBusy={!!pipelineStatus?.is_busy}
            pipelineDialogOpen={pipelineDialogOpen}
            onPipelineDialogChange={setPipelineDialogOpen}
            onRefresh={refetch}
            tenantId={selectedTenantId ?? undefined}
            workspaceId={selectedWorkspaceId ?? undefined}
          />

          {/* OODA-30: Toolbar section extracted to DocumentToolbarSection */}
          <DocumentToolbarSection
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
            statusFilter={statusFilter}
            onStatusFilterChange={setStatusFilter}
            sortField={sortField}
            onSortFieldChange={setSortField}
            sortDirection={sortDirection}
            onSortDirectionChange={setSortDirection}
            statusCounts={statusCounts}
            pipelineStatus={pipelineStatus}
            documents={documents}
            onOpenPipelineDetails={() => setPipelineDialogOpen(true)}
            getRootProps={getRootProps}
            getInputProps={getInputProps}
            isDragActive={isDragActive}
            openFileDialog={openFileDialog}
            selectedCount={selectedCount}
            onBulkReprocess={handleBulkReprocess}
            onBulkDelete={handleBulkDelete}
            onClearSelection={handleClearSelection}
            uploadingFiles={uploadingFiles}
            isUploading={isUploading}
            onRemoveUpload={removeUploadingFile}
            onUploadComplete={handleUploadComplete}
            onUploadFailed={handleUploadFailed}
          />

        </div>

      {/* OODA-26: Table section extracted to DocumentTableSection */}
      <DocumentTableSection
        documents={documents}
        totalCount={totalCount}
        isLoading={isLoading}
        selectedIds={selectedIds}
        selectedDocument={selectedDocument}
        searchQuery={searchQuery}
        statusFilter={statusFilter}
        isAllSelected={isAllSelected}
        onSelectAll={handleSelectAll}
        onSelectOne={handleSelectOne}
        onRowClick={handleDocumentClick}
        onRowDoubleClick={handleDocumentDoubleClick}
        onViewDetails={handleViewDetails}
        onViewInGraph={handleViewInGraph}
        onViewPdf={handleViewPdf}
        onRetry={(id) => reprocessMutation.mutate(id)}
        onCancel={(trackId) => cancelMutation.mutate(trackId)}
        onDelete={(id) => deleteMutation.mutate(id)}
        isRetrying={reprocessMutation.isPending}
        isCancelling={cancelMutation.isPending}
        onUploadClick={openFileDialog}
        currentPage={currentPage}
        totalPages={totalPages}
        pageSize={pageSize}
        onPageChange={setCurrentPage}
        onPageSizeChange={setPageSize}
        onClearFilter={() => {
          setStatusFilter('all');
          setSearchQuery('');
        }}
      />
      </div>

      {/* OODA-27: Right panel extracted to DocumentPreviewRightPanel */}
      <DocumentPreviewRightPanel
        isOpen={previewPanelOpen}
        onToggle={() => setPreviewPanelOpen(!previewPanelOpen)}
        onClose={handlePreviewClose}
        selectedDocument={selectedDocument}
        onDelete={(id) => deleteMutation.mutate(id)}
        onReprocess={(id) => reprocessMutation.mutate(id)}
        onViewInGraph={handleViewInGraph}
        onViewFull={(doc) => router.push(`/documents/${doc.id}`)}
        isDeleting={deleteMutation.isPending}
        isReprocessing={reprocessMutation.isPending}
        viewerDialogOpen={viewerDialogOpen}
        onViewerDialogChange={setViewerDialogOpen}
        viewerPdfId={viewerPdfId}
      />

      {/* Duplicate upload dialog — shown when backend returns duplicate_of */}
      <DuplicateUploadDialog
        open={pendingDuplicates.length > 0}
        duplicates={pendingDuplicates}
        onResolve={resolvePendingDuplicates}
      />
    </div>
  );
}

export default DocumentManager;
