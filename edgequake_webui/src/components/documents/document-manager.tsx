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
import { LargePdfAdmissionDialog } from './large-pdf-admission-dialog';
import { BulkReprocessDialog, type BulkReprocessChoice } from './bulk-reprocess-dialog';
import { ReprocessDialog, type ReprocessChoice } from './reprocess-dialog';
import { isProcessingStatus } from './status-badge';
import {
  filterLargePdfFiles,
  type LargePdfAdmissionPreview,
  type PdfParserChoice,
} from '@/lib/pdf/large-pdf-admission';
import { useCallback } from 'react';

export function DocumentManager() {
  const { t } = useTranslation();
  const router = useRouter();

  // Get tenant context for query key
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();

  // Selected document for preview panel
  const [selectedDocument, setSelectedDocument] = useState<Document | null>(null);
  const [previewPanelOpen, setPreviewPanelOpen] = useState(false);

  // Reprocess choice dialog state.
  // WHY: Reprocessing a completed PDF must let the user choose between a full
  // PDF -> markdown re-conversion (slower, spends vision tokens) and a fast
  // entity-only re-extraction (reuses cached markdown). The dialog collects the
  // intent before calling reprocessMutation with the chosen mode.
  const [reprocessTarget, setReprocessTarget] = useState<Document | null>(null);

  // Bulk reprocess choice dialog state.
  // WHY: The toolbar Reprocess button acts on every selected document at once.
  // We show one choice dialog (full vs entities) whose mode applies to the
  // whole batch, instead of prompting per document.
  const [bulkReprocessOpen, setBulkReprocessOpen] = useState(false);

  // SPEC-002: Document viewer dialog state for PDF/Markdown side-by-side view
  const [viewerDialogOpen, setViewerDialogOpen] = useState(false);
  const [viewerPdfId, setViewerPdfId] = useState<string | null>(null);

  // Search state
  const [searchQuery, setSearchQuery] = useState('');
  const [pdfParserBackend, setPdfParserBackend] = useState<'default' | 'vision' | 'edgeparse'>('default');
  const [largePdfAdmissionOpen, setLargePdfAdmissionOpen] = useState(false);
  const [largePdfPreviews, setLargePdfPreviews] = useState<LargePdfAdmissionPreview[]>([]);
  const [pendingAdmissionFiles, setPendingAdmissionFiles] = useState<File[]>([]);

  // VS-03: No pagination state — virtual scrolling handles windowing client-side.
  // We fetch all documents at once (up to VIRTUAL_PAGE_SIZE) and let the
  // virtualizer render only visible rows. This eliminates pagination UI entirely.

  // OODA-17: Filter/sort preferences with localStorage persistence
  const {
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
    pdfParserBackend:
      pdfParserBackend === 'default' ? undefined : pdfParserBackend,
  });

  const handleFilesAccepted = useCallback(
    async (files: File[]) => {
      const largePreviews = await filterLargePdfFiles(files);
      if (largePreviews.length > 0) {
        setLargePdfPreviews(largePreviews);
        setPendingAdmissionFiles(files);
        setLargePdfAdmissionOpen(true);
        return;
      }
      await handleFilesUpload(files);
    },
    [handleFilesUpload],
  );

  const handleAdmissionConfirm = useCallback(
    async (parserChoice: PdfParserChoice, files: File[]) => {
      setLargePdfAdmissionOpen(false);
      setLargePdfPreviews([]);
      setPendingAdmissionFiles([]);
      const parserOverride =
        parserChoice === 'default'
          ? undefined
          : parserChoice;
      if (parserChoice !== 'default') {
        setPdfParserBackend(parserChoice);
      }
      await handleFilesUpload(files, {
        pdfParserBackend: parserOverride,
      });
    },
    [handleFilesUpload],
  );

  const handleAdmissionCancel = useCallback(() => {
    setLargePdfAdmissionOpen(false);
    setPendingAdmissionFiles([]);
    setLargePdfPreviews([]);
  }, []);

  // OODA-14: Document mutations extracted to useDocumentMutations hook
  const {
    deleteMutation,
    reprocessMutation,
    cancelMutation,
  } = useDocumentMutations({
    onReprocessSuccess: () => setPipelineDialogOpen(true),
  });

  // OODA-29: Document queries extracted to useDocumentQueries hook
  // VS-03: page=1 with large pageSize fetches everything at once for virtual scroll
  const VIRTUAL_PAGE_SIZE = 500;
  const { data, isLoading, isError, error, refetch, pipelineStatus, queryClient } = useDocumentQueries({
    tenantId: selectedTenantId,
    workspaceId: selectedWorkspaceId,
    currentPage: 1,
    pageSize: VIRTUAL_PAGE_SIZE,
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
    onFilesAccepted: handleFilesAccepted,
    t,
  });

  // OODA-19: Filter and sort documents using extracted hook
  const { documents, totalCount, statusCounts } = useDocumentFiltering({
    documents: data?.items || [],
    searchQuery,
    statusFilter,
    sortField,
    sortDirection,
    pageSize: VIRTUAL_PAGE_SIZE,
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
  // WHY: Use document-level processing count (not task count) so the title
  // reflects what users see in the table. Tasks can be "processing" while
  // their documents are already "failed" or "completed" (e.g., after restart).
  const processingDocCount = documents?.filter(
    (d: Document) => d.status && isProcessingStatus(d.status)
  ).length ?? 0;
  useDocumentTitle({
    totalCount,
    processingCount: processingDocCount,
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
            pdfParserBackend={pdfParserBackend}
            onPdfParserBackendChange={setPdfParserBackend}
            selectedCount={selectedCount}
            onBulkReprocess={() => {
              // WHY: Open the bulk choice dialog so the user picks full
              // re-conversion vs. entity-only before reprocessing the batch.
              if (selectedCount === 0) return;
              setBulkReprocessOpen(true);
            }}
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
        onRetry={(id) => reprocessMutation.mutate({ id })}
        onReprocess={(id) => {
          // WHY: Open the choice dialog for the target document so the user can
          // pick between full PDF re-conversion and entity-only re-extraction.
          const target = documents.find((d) => d.id === id) ?? null;
          setReprocessTarget(target ?? ({ id } as Document));
        }}
        onCancel={(trackId) => cancelMutation.mutate(trackId)}
        onDelete={(id) => deleteMutation.mutate(id)}
        isRetrying={reprocessMutation.isPending}
        isCancelling={cancelMutation.isPending}
        onUploadClick={openFileDialog}
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
        onReprocess={(id) => {
          // WHY: Open the choice dialog for the target document so the user can
          // pick between full re-conversion and entity-only re-extraction. For
          // non-PDF docs the dialog still shows but the mode only affects PDFs.
          const target = documents.find((d) => d.id === id) ?? null;
          setReprocessTarget(target ?? ({ id } as Document));
        }}
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

      <LargePdfAdmissionDialog
        open={largePdfAdmissionOpen}
        previews={largePdfPreviews}
        onOpenChange={setLargePdfAdmissionOpen}
        onConfirm={handleAdmissionConfirm}
        onCancel={handleAdmissionCancel}
      />

      {/* Reprocess choice dialog — lets the user choose full PDF re-conversion
          vs. entity-only re-extraction before queueing the reprocess task. */}
      <ReprocessDialog
        open={reprocessTarget !== null}
        document={reprocessTarget}
        onConfirm={(choice: ReprocessChoice) => {
          if (!reprocessTarget?.id) return;
          reprocessMutation.mutate({ id: reprocessTarget.id, mode: choice.mode });
          setReprocessTarget(null);
        }}
        onCancel={() => setReprocessTarget(null)}
      />

      {/* Bulk reprocess choice dialog — one mode applied to all selected docs. */}
      <BulkReprocessDialog
        open={bulkReprocessOpen}
        count={selectedCount}
        onConfirm={(choice: BulkReprocessChoice) => {
          setBulkReprocessOpen(false);
          void handleBulkReprocess(choice.mode);
        }}
        onCancel={() => setBulkReprocessOpen(false)}
      />
    </div>
  );
}

export default DocumentManager;
