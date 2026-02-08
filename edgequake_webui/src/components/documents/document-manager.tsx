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

import { RightPanel } from '@/components/layout/right-panel';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import {
    Table,
    TableBody,
    TableHead,
    TableHeader,
    TableRow,
} from '@/components/ui/table';
import {
    getDocuments,
    getPipelineStatus,
} from '@/lib/api/edgequake';

import { useTenantStore } from '@/stores/use-tenant-store';
import type { Document } from '@/types';
import { useQuery, useQueryClient } from '@tanstack/react-query';

import {
    AlertCircle,
    FileText,
    Loader2,
    RefreshCw,
    Search,
    X,
} from 'lucide-react';
import { useRouter } from 'next/navigation';
import { useCallback, useEffect, useState } from 'react';
import { useDropzone } from 'react-dropzone';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { ClearDocumentsDialog } from './clear-documents-dialog';
import { BatchActionsBar } from './batch-actions-bar';
import { ConnectionBanner } from './connection-banner';
import { ConnectionStatus } from './connection-status';

import { DocumentDropzone } from './document-dropzone';
import { DocumentTableStates } from './document-table-states';

import { DocumentFilters } from './document-filters';
import { DocumentPreviewPanel } from './document-preview-panel';
import { DocumentTableRow } from './document-table-row';
import { DocumentViewerDialog } from './document-viewer-dialog';

import { PaginationControls } from './pagination-controls';
import { PipelineStatusDialog } from './pipeline-status-dialog';
import { ProcessingStatusSummary } from './processing-status-summary';
import { ReprocessFailedButton } from './reprocess-failed-button';
import { UploadProgressList } from './upload-progress-list';
import { useStuckDetection } from '@/hooks/use-stuck-detection';
import { useDocumentWebSocket } from '@/hooks/use-document-websocket';
import { useFileUpload } from '@/hooks/use-file-upload';
import { useDocumentMutations } from '@/hooks/use-document-mutations';
import { useBulkSelection } from '@/hooks/use-bulk-selection';
import { useDocumentPreferences, type DocStatus } from '@/hooks/use-document-preferences';
import { useDocumentKeyboard } from '@/hooks/use-document-keyboard';
import { useDocumentFiltering } from '@/hooks/use-document-filtering';

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

  // Maximum file size: 10MB
  const MAX_FILE_SIZE = 10 * 1024 * 1024;

  const onDrop = useCallback(
    async (acceptedFiles: File[], fileRejections: readonly { file: File; errors: readonly { code: string; message: string }[] }[]) => {
      // Handle rejected files (too large or wrong type)
      for (const rejection of fileRejections) {
        const errorMessages = rejection.errors.map(e => {
          if (e.code === 'file-too-large') {
            const sizeMB = (rejection.file.size / (1024 * 1024)).toFixed(2);
            return t('documents.upload.fileTooLarge', 'File "{{name}}" is too large ({{size}}MB). Maximum size is 10MB.', {
              name: rejection.file.name,
              size: sizeMB,
            });
          }
          if (e.code === 'file-invalid-type') {
            return t('documents.upload.invalidType', 'File "{{name}}" has an unsupported format. Supported: TXT, MD, JSON, PDF.', {
              name: rejection.file.name,
            });
          }
          return e.message;
        }).join(', ');
        
        toast.error(errorMessages);
      }
      
      // Process accepted files
      if (acceptedFiles.length > 0) {
        await handleFilesUpload(acceptedFiles);
      }
    },
    [handleFilesUpload, t]
  );

  const { getRootProps, getInputProps, isDragActive, open: openFileDialog } = useDropzone({
    onDrop,
    accept: {
      'text/plain': ['.txt'],
      'text/markdown': ['.md'],
      'application/json': ['.json'],
      'application/pdf': ['.pdf'],
    },
    maxSize: MAX_FILE_SIZE, // 10MB limit
    noClick: false, // Allow click on dropzone
  });

  // OODA-19: Filter and sort documents using extracted hook
  const { documents, totalCount, totalPages, allDocuments } = useDocumentFiltering({
    documents: data?.items || [],
    searchQuery,
    statusFilter,
    sortField,
    sortDirection,
    pageSize,
  });
  
  // Use server-side status counts from API response (more efficient)
  // Fall back to client-side calculation if not available
  const serverStatusCounts = data?.status_counts;
  const statusCounts: Record<DocStatus, number> = serverStatusCounts ? {
    all: allDocuments.length,
    pending: serverStatusCounts.pending,
    processing: serverStatusCounts.processing,
    completed: serverStatusCounts.completed,
    failed: serverStatusCounts.failed,
    partial_failure: serverStatusCounts.partial_failure || 0,
    cancelled: serverStatusCounts.cancelled || 0,
  } : {
    all: allDocuments.length,
    pending: allDocuments.filter((d) => d.status === 'pending').length,
    processing: allDocuments.filter((d) => d.status === 'processing').length,
    completed: allDocuments.filter((d) => !d.status || d.status === 'completed' || d.status === 'indexed').length,
    failed: allDocuments.filter((d) => d.status === 'failed').length,
    partial_failure: allDocuments.filter((d) => d.status === 'partial_failure').length,
    cancelled: allDocuments.filter((d) => d.status === 'cancelled').length,
  };

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

  /**
   * OODA-26: Update page title with document count
   * WHY: Users can see document count without switching tabs
   */
  useEffect(() => {
    const baseTitle = 'Documents - EdgeQuake';
    const count = totalCount || 0;
    const processing = pipelineStatus?.running_tasks || 0;
    
    if (processing > 0) {
      document.title = `⏳ Processing (${processing}) | Documents (${count}) - EdgeQuake`;
    } else if (count > 0) {
      document.title = `Documents (${count}) - EdgeQuake`;
    } else {
      document.title = baseTitle;
    }
    
    return () => { document.title = baseTitle; };
  }, [totalCount, pipelineStatus?.running_tasks]);

  if (isError) {
    return (
      <div className="p-6">
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertTitle>Error loading documents</AlertTitle>
          <AlertDescription>
            {error instanceof Error ? error.message : 'Failed to load documents'}
            <Button variant="link" className="ml-2 p-0" onClick={() => refetch()}>
              Try again
            </Button>
          </AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div className="flex h-full overflow-hidden">
      {/* Main Content - Flex column for proper scroll zones */}
      <div className="flex-1 flex flex-col min-h-0 overflow-hidden">
        {/* Fixed Header Zone */}
        <div className="shrink-0 px-4 pt-4 space-y-3 bg-background">
          {/* OODA-02: Connection status banner when disconnected */}
          <ConnectionBanner />
          
          {/* Header - Compact */}
          <header className="flex items-center justify-between gap-3 flex-wrap">
            <div className="space-y-0.5">
              <div className="flex items-center gap-2">
                <h1 className="text-xl font-semibold tracking-tight">{t('documents.title')}</h1>
                {/* OODA-39: Document count badge */}
                {totalCount > 0 && (
                  <Badge variant="secondary" className="text-xs font-normal">
                    {totalCount}
                  </Badge>
                )}
                {/* OODA-30: WebSocket connection status indicator */}
                <ConnectionStatus compact={true} />
              </div>
              <p className="text-sm text-muted-foreground">
                {t('documents.subtitle')}
              </p>
            </div>
          <div className="flex items-center gap-2 flex-wrap">
            {/* Pipeline Status */}
            {pipelineStatus?.is_busy && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPipelineDialogOpen(true)}
                className="gap-1 text-orange-500"
              >
                <Loader2 className="h-4 w-4 animate-spin" />
                {t('pipeline.busy')}
              </Button>
            )}
            <PipelineStatusDialog
              open={pipelineDialogOpen}
              onOpenChange={setPipelineDialogOpen}
              tenantId={selectedTenantId ?? undefined}
              workspaceId={selectedWorkspaceId ?? undefined}
            />
            
            {/* Reprocess Failed Button (GAP-UI-002) */}
            <ReprocessFailedButton
              failedCount={statusCounts.failed}
              onReprocessStarted={(trackId) => {
                setPipelineDialogOpen(true);
              }}
            />
          
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4 mr-1" />
            {t('documents.refresh')}
          </Button>
          
          {/* Clear Documents Dialog (GAP-UI-009) */}
          <ClearDocumentsDialog
            documentCount={totalCount}
            onCleared={() => refetch()}
          />
        </div>
      </header>
      
      {/* Search and Filters */}
      <div className="flex flex-col sm:flex-row sm:items-center gap-3 pb-3 border-b">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={t('documents.search.placeholder', 'Search documents...')}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9 pr-8 h-9"
          />
          {/* OODA-36: Clear search button */}
          {searchQuery && (
            <button
              type="button"
              onClick={() => setSearchQuery('')}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-1 rounded hover:bg-muted transition-colors"
              aria-label="Clear search"
            >
              <X className="h-3.5 w-3.5 text-muted-foreground" />
            </button>
          )}
        </div>
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

      {/* Scrollable Documents Table Zone */}
      <div className="flex-1 min-h-0 overflow-auto">
        <div className="px-4 py-3">
          {/* Table Header */}
          <div className="flex items-center gap-2 mb-3">
            <FileText className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm font-medium">Documents ({documents.length})</span>
          </div>
          
          {/* OODA-12: Loading skeleton and empty state - Extracted to DocumentTableStates */}
          <DocumentTableStates
            isLoading={isLoading}
            isEmpty={documents.length === 0}
            onUploadClick={openFileDialog}
          />
          
          {!isLoading && documents.length > 0 && (
            <div className="border rounded-lg overflow-hidden shadow-sm">
              <Table>
                <TableHeader className="bg-muted/50 sticky top-0 z-10">
                  <TableRow className="hover:bg-transparent">
                    <TableHead className="w-[40px]">
                      <Checkbox
                        checked={isAllSelected}
                        onCheckedChange={(checked) => handleSelectAll(!!checked)}
                        aria-label={t('documents.bulk.selectAll', 'Select all')}
                      />
                    </TableHead>
                    <TableHead>Title</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="text-center">Entities</TableHead>
                    <TableHead className="text-center">Cost</TableHead>
                    <TableHead>Created</TableHead>
                    <TableHead className="w-[100px]"></TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {/* OODA-15: Table rows extracted to DocumentTableRow component */}
                  {documents.map((doc, index) => (
                    <DocumentTableRow
                      key={doc.id}
                      doc={doc}
                      index={index}
                      isSelected={selectedIds.has(doc.id)}
                      isActive={selectedDocument?.id === doc.id}
                      searchQuery={searchQuery}
                      onSelect={handleSelectOne}
                      onClick={handleDocumentClick}
                      onDoubleClick={handleDocumentDoubleClick}
                      onViewDetails={handleViewDetails}
                      onViewInGraph={handleViewInGraph}
                      onViewPdf={handleViewPdf}
                      onRetry={(id) => reprocessMutation.mutate(id)}
                      onCancel={(trackId) => cancelMutation.mutate(trackId)}
                      onDelete={(id) => deleteMutation.mutate(id)}
                      isRetrying={reprocessMutation.isPending}
                      isCancelling={cancelMutation.isPending}
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
          {/* OODA-37: Show filtered vs total count when filtering */}
          {(searchQuery || statusFilter !== 'all') && (
            <p className="text-xs text-muted-foreground mb-2 text-center">
              Showing {documents.length} of {totalCount} documents
              {statusFilter !== 'all' && ` (${statusFilter})`}
              {searchQuery && ` matching "${searchQuery}"`}
            </p>
          )}
          <PaginationControls
            currentPage={currentPage}
            totalPages={totalPages}
            pageSize={pageSize}
            onPageChange={setCurrentPage}
            onPageSizeChange={(newSize) => {
              setPageSize(newSize);
              setCurrentPage(1);
            }}
          />
        </div>
      )}
      </div>

      {/* Right Panel - Document Preview */}
      <RightPanel
        isOpen={previewPanelOpen}
        onToggle={() => setPreviewPanelOpen(!previewPanelOpen)}
        onClose={handlePreviewClose}
        title={selectedDocument ? (selectedDocument.title || selectedDocument.file_name || `Document ${selectedDocument.id.slice(0, 8)}`) : t('documents.preview.title', 'Document Preview')}
        subtitle={selectedDocument?.id ? `ID: ${selectedDocument.id.slice(0, 12)}...` : undefined}
        width="wide"
        showCollapsedBar={true}
        collapsedLabel={t('documents.preview.panelLabel', 'Preview')}
        headerIcon={<FileText className="h-4 w-4" />}
      >
        <DocumentPreviewPanel
          document={selectedDocument}
          onDelete={(id) => {
            deleteMutation.mutate(id);
            handlePreviewClose();
          }}
          onReprocess={(id) => reprocessMutation.mutate(id)}
          onViewFull={(doc) => {
            // OODA-41: Always navigate to document detail page
            // WHY: Per SPEC-002, use dedicated page instead of dialog
            router.push(`/documents/${doc.id}`);
          }}
          onViewInGraph={handleViewInGraph}
          isDeleting={deleteMutation.isPending}
          isReprocessing={reprocessMutation.isPending}
        />
      </RightPanel>

      {/* SPEC-002: PDF/Markdown Viewer Dialog */}
      <DocumentViewerDialog
        open={viewerDialogOpen}
        onOpenChange={setViewerDialogOpen}
        pdfId={viewerPdfId}
      />
    </div>
  );
}

export default DocumentManager;
