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

import {
    getDocuments,
    getPipelineStatus,
} from '@/lib/api/edgequake';

import { useTenantStore } from '@/stores/use-tenant-store';
import type { Document } from '@/types';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useRouter } from 'next/navigation';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { BatchActionsBar } from './batch-actions-bar';

import { DocumentDropzone } from './document-dropzone';
import { DocumentErrorAlert } from './document-error-alert';
import { DocumentHeader } from './document-header';

import { DocumentFilters } from './document-filters';
import { DocumentPreviewRightPanel } from './document-preview-right-panel';
import { DocumentSearchBar } from './document-search-bar';
import { DocumentTableSection } from './document-table-section';

import { ProcessingStatusSummary } from './processing-status-summary';
import { UploadProgressList } from './upload-progress-list';
import { useStuckDetection } from '@/hooks/use-stuck-detection';
import { useDocumentWebSocket } from '@/hooks/use-document-websocket';
import { useFileUpload } from '@/hooks/use-file-upload';
import { useDocumentMutations } from '@/hooks/use-document-mutations';
import { useBulkSelection } from '@/hooks/use-bulk-selection';
import { useDocumentPreferences } from '@/hooks/use-document-preferences';
import { useDocumentKeyboard } from '@/hooks/use-document-keyboard';
import { useDocumentFiltering } from '@/hooks/use-document-filtering';
import { useDocumentDropzone } from '@/hooks/use-document-dropzone';
import { useDocumentTitle } from '@/hooks/use-document-title';

export function DocumentManager() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const router = useRouter();
  
  // Get tenant context for query key
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // CRITICAL DEBUG: Log tenant/workspace changes
  useEffect(() => {
    console.log('[DocumentManager] Tenant/Workspace context:', {
      selectedTenantId,
      selectedWorkspaceId,
      timestamp: new Date().toISOString(),
    });
  }, [selectedTenantId, selectedWorkspaceId]);
  
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
  } = useDocumentMutations({
    onReprocessSuccess: () => setPipelineDialogOpen(true),
  });

  // OODA-42 COMPLETE: WebSocket-based real-time updates (NO POLLING)
  // WHY: Users want instant document status updates without polling overhead
  // HOW: Subscribe to WebSocket events for all processing documents
  const { data, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['documents', selectedTenantId, selectedWorkspaceId, currentPage, pageSize, statusFilter],
    queryFn: () => getDocuments({ 
      page: currentPage, 
      page_size: pageSize,
      status: statusFilter === 'all' ? undefined : statusFilter,
    }),
    // NO polling - WebSocket provides real-time updates
    refetchInterval: false,
  });
  
  // Pipeline status query
  // OODA-37: Include workspace in queryKey for proper isolation
  // CRITICAL: Pass tenant_id and workspace_id to getPipelineStatus for multi-tenancy isolation
  const { data: pipelineStatus } = useQuery({
    queryKey: ['pipeline-status', selectedTenantId, selectedWorkspaceId],
    queryFn: () => getPipelineStatus(selectedTenantId ?? undefined, selectedWorkspaceId ?? undefined),
    refetchInterval: 2000,
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

  // Document selection for preview panel
  const handleDocumentClick = useCallback((doc: Document) => {
    setSelectedDocument(doc);
    setPreviewPanelOpen(true);
  }, []);

  /**
   * OODA-41: Double-click to navigate to document detail page
   * WHY: Power users expect double-click for primary navigation action
   * SPEC-002: Navigate to dedicated document detail page, not dialog
   */
  const handleDocumentDoubleClick = useCallback((doc: Document) => {
    router.push(`/documents/${doc.id}`);
  }, [router]);

  /**
   * OODA-41: Navigate to document detail page (for View Details button)
   * WHY: Users need explicit link to dedicated document view
   */
  const handleViewDetails = useCallback((doc: Document) => {
    router.push(`/documents/${doc.id}`);
  }, [router]);

  const handlePreviewClose = useCallback(() => {
    setSelectedDocument(null);
    setPreviewPanelOpen(false);
  }, []);

  const handleViewInGraph = useCallback((doc: Document) => {
    router.push(`/graph?entity=${encodeURIComponent(doc.id)}`);
  }, [router]);

  /**
   * SPEC-002: Open PDF viewer dialog for PDF documents
   * WHY: Users need to view original PDF alongside extracted markdown
   */
  const handleViewPdf = useCallback((doc: Document) => {
    // Use pdf_id if available, otherwise try to derive from source_type
    const pdfId = doc.pdf_id || (doc.source_type === 'pdf' ? doc.id : null);
    if (pdfId) {
      setViewerPdfId(pdfId);
      setViewerDialogOpen(true);
    } else {
      // Fallback to standard document view
      router.push(`/documents/${doc.id}`);
    }
  }, [router]);

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
            failedCount={statusCounts.failed}
            pipelineIsBusy={!!pipelineStatus?.is_busy}
            pipelineDialogOpen={pipelineDialogOpen}
            onPipelineDialogChange={setPipelineDialogOpen}
            onRefresh={refetch}
            tenantId={selectedTenantId ?? undefined}
            workspaceId={selectedWorkspaceId ?? undefined}
          />
      
      {/* Search and Filters */}
      <div className="flex flex-col sm:flex-row sm:items-center gap-3 pb-3 border-b">
        <DocumentSearchBar
          value={searchQuery}
          onChange={setSearchQuery}
        />
        <DocumentFilters
          status={statusFilter}
          onStatusChange={setStatusFilter}
          sortField={sortField}
          onSortFieldChange={setSortField}
          sortDirection={sortDirection}
          onSortDirectionChange={setSortDirection}
          statusCounts={statusCounts}
        />
      </div>

      {/* OODA-11: Processing Status Summary - Extracted to ProcessingStatusSummary component */}
      {pipelineStatus && (
        <ProcessingStatusSummary
          pipelineStatus={pipelineStatus}
          documents={documents}
          onOpenDetails={() => setPipelineDialogOpen(true)}
        />
      )}

      {/* OODA-08: Compact Upload Zone - Extracted to DocumentDropzone component */}
      <DocumentDropzone
        getRootProps={getRootProps}
        getInputProps={getInputProps}
        isDragActive={isDragActive}
      />

      {/* OODA-07: Bulk Actions Bar - Extracted to BatchActionsBar component */}
      <BatchActionsBar
        selectedCount={selectedCount}
        onReprocess={handleBulkReprocess}
        onDelete={handleBulkDelete}
        onClear={handleClearSelection}
      />

      {/* OODA-06: Upload Progress - Extracted to UploadProgressList component */}
      <UploadProgressList
        uploadingFiles={uploadingFiles}
        isUploading={isUploading}
        onRemove={removeUploadingFile}
        onComplete={handleUploadComplete}
        onFailed={handleUploadFailed}
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
    </div>
  );
}

export default DocumentManager;
