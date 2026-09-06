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

import { useSelectedWorkspace, useTenantStore } from '@/stores/use-tenant-store';
import type { Document } from '@/types';
import {
  DEFAULT_SECURITY_FIELDS,
  type SecurityFields,
} from '@/lib/security/security-fields';
import { patchDocumentSecurityLabels } from '@/lib/api/edgequake/documents';
import type { ClassificationFilter } from './document-filters';

import { useRouter } from 'next/navigation';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { SecurityFieldsForm } from '@/components/security/security-fields-form';
import { nextDocumentSortState } from '@/lib/documents/document-sort';
import { documentsChromeMaxClass } from '@/lib/documents/documents-chrome';
import {
  beginDeleteSession,
  bindDeleteSessionTrackId,
  dismissDeleteSession,
  formatDeleteCountsLabel,
  formatDeleteProgressHeader,
  formatDeleteStageMessage,
  getActiveDeletingDocumentIds,
  patchDocumentsDeletingOptimistic,
} from '@/lib/documents/deletion-session';
import {
  resolveReprocessPanelTrackId,
  shouldShowReprocessQueuingPanel,
  unpinReprocessDocuments,
} from '@/lib/documents/progress-admit';
import { stageDisplayName } from '@/lib/pipeline/ingestion-run-view';
import {
  FEEDBACK_ZONE_RESERVE_MIN_PX,
  readLiveWorkHint,
  shouldReserveFeedbackSlot,
  writeLiveWorkHint,
} from '@/lib/documents/documents-layout-stability';
import {
  needsReuploadNotReprocess,
  resolvePipelineUiState,
} from '@/lib/pipeline/pipeline-document-state';

import { useBulkSelection } from '@/hooks/use-bulk-selection';
import { useDeletionSessions } from '@/hooks/use-deletion-progress';
import { useDocumentDropzone } from '@/hooks/use-document-dropzone';
import { useDocumentHandlers } from '@/hooks/use-document-handlers';
import { useDocumentKeyboard } from '@/hooks/use-document-keyboard';
import { useDocumentMutations } from '@/hooks/use-document-mutations';
import { useDocumentPreferences } from '@/hooks/use-document-preferences';
import { useDocumentsInventory } from '@/hooks/use-documents-inventory';
import { useDocumentTitle } from '@/hooks/use-document-title';
import { useDocumentWebSocket } from '@/hooks/use-document-websocket';
import { useDocAbacEnabled } from '@/hooks/use-doc-abac';
import {
  DEFAULT_VISION_EXTRACT_DRAFT,
  type VisionExtractDraft,
} from '@/components/settings/vision-extract-controls';
import { useFileUpload } from '@/hooks/use-file-upload';
import { useLiveWorkControllers } from '@/hooks/use-live-work-controllers';
import {
  shouldUsePdfReprocessPanel,
  useReprocessTracking,
} from '@/hooks/use-reprocess-tracking';
import { useStuckDetection } from '@/hooks/use-stuck-detection';
import type { PdfParserResolutionContext } from '@/lib/pdf/large-pdf-admission';
import {
    filterLargePdfFiles,
    type LargePdfAdmissionPreview,
    type PdfParserChoice,
} from '@/lib/pdf/large-pdf-admission';
import { AdmissionPhaseRow } from './admission-phase-row';
import { ActiveRunsPanel } from './active-runs-panel';
import { BulkDeleteConfirmDialog } from './bulk-delete-confirm-dialog';
import { BulkReprocessDialog, type BulkReprocessChoice } from './bulk-reprocess-dialog';
import { DeleteConfirmDialog } from './delete-confirm-dialog';
import { DocumentErrorAlert } from './document-error-alert';
import { DocumentHeader } from './document-header';
import { DocumentPreviewRightPanel } from './document-preview-right-panel';
import { DocumentsActionsProvider } from './documents-actions-context';
import { DocumentTableSection } from './document-table-section';
import { PaginationControls } from './pagination-controls';
import { DocumentToolbarSection } from './document-toolbar-section';
import { DuplicateUploadDialog } from './duplicate-upload-dialog';
import { FeedbackZoneLiveRegion } from './feedback-zone-live-region';
import { FeedbackZoneSkeleton } from './feedback-zone-skeleton';
import { LargePdfAdmissionDialog } from './large-pdf-admission-dialog';
import { ProgressPanelRow } from './progress-panel-row';
import { ReprocessDialog, type ReprocessChoice } from './reprocess-dialog';
import { ApiErrorBoundary } from '@/components/shared/api-error-boundary';
import { Button } from '@/components/ui/button';
import { DropdownMenuCheckboxItem } from '@/components/ui/dropdown-menu';
import { UploadProgressList } from './upload-progress-list';
import { X } from 'lucide-react';

export function DocumentManager() {
  const { t } = useTranslation();
  const router = useRouter();

  // Get tenant context for query key
  const { selectedTenantId, selectedWorkspaceId } = useTenantStore();
  const selectedWorkspace = useSelectedWorkspace();

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

  // SPEC-050 GAP-FIX: Delete confirm dialog state.
  // WHY: Both single (preview panel) and bulk (toolbar) delete routes must
  // open a confirmation dialog before deleting. This single state drives both.
  const [deleteConfirmTarget, setDeleteConfirmTarget] = useState<Document | null>(null);
  const [bulkDeleteTargets, setBulkDeleteTargets] = useState<Document[]>([]);
  const [bulkDeleteDialogOpen, setBulkDeleteDialogOpen] = useState(false);

  // SPEC-002: Document viewer dialog state for PDF/Markdown side-by-side view
  const [viewerDialogOpen, setViewerDialogOpen] = useState(false);
  const [viewerPdfId, setViewerPdfId] = useState<string | null>(null);

  // Search state
  const [searchQuery, setSearchQuery] = useState('');
  const [pdfParserBackend, setPdfParserBackend] = useState<
    'default' | 'vision' | 'edgeparse' | 'auto'
  >('default');
  const [visionReasoningEffort, setVisionReasoningEffort] = useState<string | undefined>();
  const [visionExtract, setVisionExtract] = useState<VisionExtractDraft>(
    () => ({ ...DEFAULT_VISION_EXTRACT_DRAFT }),
  );
  const [securityFields, setSecurityFields] = useState<SecurityFields>(
    () => ({ ...DEFAULT_SECURITY_FIELDS }),
  );
  const [securityExpanded, setSecurityExpanded] = useState(false);
  const handleSecurityExpandedChange = useCallback((open: boolean) => {
    setSecurityExpanded(open);
    if (open) {
      // After layout grows chrome, bring SecurityFields into the chrome viewport.
      requestAnimationFrame(() => {
        document
          .querySelector('[data-testid="spec146-security-fields"]')
          ?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      });
    }
  }, []);
  const { docAbacEnabled } = useDocAbacEnabled();
  const [classificationFilter, setClassificationFilter] =
    useState<ClassificationFilter>('all');
  const [retryingLabels, setRetryingLabels] = useState(false);
  const [retryLabelsDoc, setRetryLabelsDoc] = useState<Document | null>(null);
  const [retryLabelFields, setRetryLabelFields] = useState<SecurityFields>(
    DEFAULT_SECURITY_FIELDS,
  );
  const [largePdfAdmissionOpen, setLargePdfAdmissionOpen] = useState(false);
  const [largePdfPreviews, setLargePdfPreviews] = useState<LargePdfAdmissionPreview[]>([]);
  const [pendingAdmissionFiles, setPendingAdmissionFiles] = useState<File[]>([]);

  const pdfParserResolutionContext = useMemo<PdfParserResolutionContext>(
    () => ({
      uploadChoice: pdfParserBackend,
      workspaceBackend: selectedWorkspace?.pdf_parser_backend,
    }),
    [pdfParserBackend, selectedWorkspace?.pdf_parser_backend],
  );

  useEffect(() => {
    if (!selectedWorkspace) return;
    setVisionExtract({
      extractImages: selectedWorkspace.vision_extract_images ?? true,
      extractCharts: selectedWorkspace.vision_extract_charts ?? true,
      extractFigures: selectedWorkspace.vision_extract_figures ?? true,
      pageSystemPrompt: selectedWorkspace.vision_page_system_prompt ?? '',
      imageSystemPrompt: selectedWorkspace.vision_image_system_prompt ?? '',
      chartSystemPrompt: selectedWorkspace.vision_chart_system_prompt ?? '',
      figureSystemPrompt: selectedWorkspace.vision_figure_system_prompt ?? '',
    });
  }, [selectedWorkspace?.id]);

  // SPEC-141: inventory owns API page + page-size; virtualizer still windows the page.

  // OODA-17: Filter/sort preferences with localStorage persistence
  const {
    statusFilter, setStatusFilter,
    sortField, setSortField,
    sortDirection, setSortDirection,
    showCostColumn, setShowCostColumn,
  } = useDocumentPreferences();

  const handleColumnSort = useCallback(
    (field: typeof sortField) => {
      const next = nextDocumentSortState(sortField, sortDirection, field);
      setSortField(next.field);
      setSortDirection(next.direction);
    },
    [sortField, sortDirection, setSortField, setSortDirection],
  );

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
    pruneTerminalUploads,
    pendingDuplicates,
    resolvePendingDuplicates,
  } = useFileUpload({
    tenantId: selectedTenantId,
    workspaceId: selectedWorkspaceId,
    onUploadStart: () => setStatusFilter('all'),
    pdfParserBackend:
      pdfParserBackend === 'default' ? undefined : pdfParserBackend,
    visionReasoningEffort,
    visionExtractImages: visionExtract.extractImages,
    visionExtractCharts: visionExtract.extractCharts,
    visionExtractFigures: visionExtract.extractFigures,
    visionPageSystemPrompt: visionExtract.pageSystemPrompt || undefined,
    visionImageSystemPrompt: visionExtract.imageSystemPrompt || undefined,
    visionChartSystemPrompt: visionExtract.chartSystemPrompt || undefined,
    visionFigureSystemPrompt: visionExtract.figureSystemPrompt || undefined,
    security: docAbacEnabled ? securityFields : undefined,
  });

  const handleFilesAccepted = useCallback(
    async (files: File[]) => {
      const largePreviews = await filterLargePdfFiles(files, pdfParserResolutionContext);
      if (largePreviews.length > 0) {
        setLargePdfPreviews(largePreviews);
        setPendingAdmissionFiles(files);
        setLargePdfAdmissionOpen(true);
        return;
      }
      await handleFilesUpload(files);
    },
    [handleFilesUpload, pdfParserResolutionContext],
  );

  const handleAdmissionConfirm = useCallback(
    async (parserChoice: PdfParserChoice, files: File[]) => {
      setLargePdfAdmissionOpen(false);
      setLargePdfPreviews([]);
      setPendingAdmissionFiles([]);

      // SPEC-123 LAW-123-6: admission override applies only to large PDFs.
      const largeNames = new Set(
        largePdfPreviews.map((preview) => preview.file.name),
      );
      const largeFiles = files.filter((file) => largeNames.has(file.name));
      const otherFiles = files.filter((file) => !largeNames.has(file.name));

      const parserOverride =
        parserChoice === 'default' ? undefined : parserChoice;
      if (parserChoice !== 'default') {
        setPdfParserBackend(parserChoice);
      }

      if (otherFiles.length > 0) {
        await handleFilesUpload(otherFiles);
      }
      if (largeFiles.length > 0) {
        await handleFilesUpload(largeFiles, {
          pdfParserBackend: parserOverride,
        });
      }
    },
    [handleFilesUpload, largePdfPreviews],
  );

  const handleAdmissionCancel = useCallback(() => {
    setLargePdfAdmissionOpen(false);
    setPendingAdmissionFiles([]);
    setLargePdfPreviews([]);
  }, []);

  // SPEC-050-REPROCESS: Track reprocess operations to show ProgressPanelRow / IngestionRunCard
  // — identical feedback to a fresh upload (stage list, cost, ETA, cancel).
  // WHY SRP: This hook owns only state; the rendering and the mutation callback
  // are wired below, keeping each concern in the right layer.
  const {
    reprocessEntries,
    addReprocessEntry,
    removeReprocessEntry,
    removeReprocessEntryByDocumentId,
    pruneTerminalReprocessEntries,
  } = useReprocessTracking();

  // OODA-14: Document mutations extracted to useDocumentMutations hook
  const {
    deleteMutation,
    reprocessMutation,
    cancelMutation,
  } = useDocumentMutations({
    onReprocessSuccess: () => setPipelineDialogOpen(true),
    // SPEC-051: Forward documentId + isPdf + mode so the tracking layer can
    // use the stable document ID for prune logic and component selection.
    onReprocessTriggered: (name, trackId, opts) =>
      addReprocessEntry(name, trackId, opts),
    onReprocessDismissed: (documentId) =>
      removeReprocessEntryByDocumentId(documentId),
  });

  // Feedback-zone delete sessions (WS phase updates).
  // SPEC-098 LAW-098-10: table dimming derives from sessions (one SSOT).
  const deleteSessions = useDeletionSessions();
  const deleteProgressHeader = useMemo(
    () => formatDeleteProgressHeader(deleteSessions),
    [deleteSessions],
  );
  const deletingDocumentIds = useMemo(
    () => getActiveDeletingDocumentIds(),
    [deleteSessions],
  );

  // SPEC-069: tick so long graph-phase "Still working…" updates without new WS.
  const [deleteNow, setDeleteNow] = useState(() => Date.now());
  useEffect(() => {
    const needsTick = deleteSessions.some(
      (s) =>
        s.status === 'active' &&
        ((s.phase ?? '').toLowerCase() === 'removing_graph' ||
          s.phaseLabel.toLowerCase().includes('graph')),
    );
    if (!needsTick) return;
    const id = window.setInterval(() => setDeleteNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [deleteSessions]);

  // SPEC-099: inventory controller (queries + filter VM + overflow honesty)
  const {
    data,
    isLoading,
    isFetching,
    isError,
    error,
    refetch,
    pipelineStatus,
    queryClient,
    documents,
    totalCount,
    totalPages,
    currentPage,
    setCurrentPage,
    pageSize,
    setPageSize,
    statusCounts,
    inventory,
  } = useDocumentsInventory({
    tenantId: selectedTenantId,
    workspaceId: selectedWorkspaceId,
    searchQuery,
    statusFilter,
    classificationFilter,
    sortField,
    sortDirection,
  });

  // CLS: remember prior live-work so refresh can reserve the feedback slot early.
  const [liveWorkHint, setLiveWorkHint] = useState(false);
  useEffect(() => {
    setLiveWorkHint(readLiveWorkHint());
  }, []);

  /**
   * SPEC-050: Paint-first delete session + optimistic badge, then mutate.
   * Feedback zone narrates WS phases; toast is no longer the primary surface.
   */
  const handleDeleteDocument = useCallback(
    (id: string) => {
      const items = (data as { items?: Document[] } | undefined)?.items;
      const doc = items?.find((d) => d.id === id);
      const name = doc?.file_name || doc?.title || id.slice(0, 8);
      beginDeleteSession({ documentId: id, documentName: name });
      patchDocumentsDeletingOptimistic(queryClient, id);
      deleteMutation.mutate(id);
    },
    [data, deleteMutation, queryClient],
  );

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

  // SPEC-048: clear upload chrome when documents reach terminal state
  useEffect(() => {
    pruneTerminalUploads(documents ?? []);
    // SPEC-050-REPROCESS: also prune reprocess progress panels on terminal state
    pruneTerminalReprocessEntries(documents ?? []);
  }, [documents, pruneTerminalUploads, pruneTerminalReprocessEntries]);

  // SPEC-099: single shared pipeline UI resolve (shell → toolbar)
  const pipelineUi = useMemo(
    () => resolvePipelineUiState(documents, pipelineStatus),
    [documents, pipelineStatus],
  );

  const stuckDocIds = useMemo(
    () => new Set(pipelineUi.stuckDocs.map((d) => d.id)),
    [pipelineUi.stuckDocs],
  );

  const {
    hasLiveWork: feedbackZoneOpen,
    isLiveRunIds: workingRunDocumentIds,
    showActiveRuns,
    showUploadList,
    activeRunsDisplayed,
    uploadFilesForList,
    sessionReprocessEntries,
  } = useLiveWorkControllers({
    documents,
    pipelineStatus,
    uploadingFiles,
    reprocessEntries,
    deleteSessionCount: deleteSessions.length,
    pipelineUiAlertMode: pipelineUi.alertMode,
    stuckDocIds,
  });

  const isInitialLoading = isLoading && !data;
  const reserveFeedbackSlot = shouldReserveFeedbackSlot({
    hasLiveWork: feedbackZoneOpen,
    isInitialLoading,
    pipelineStatus,
    liveWorkHint,
  });
  const showFeedbackZone = feedbackZoneOpen || reserveFeedbackSlot;

  useEffect(() => {
    if (feedbackZoneOpen) {
      writeLiveWorkHint(true);
      setLiveWorkHint(true);
      return;
    }
    // Clear hint only after a settled idle paint (not mid-fetch).
    if (!isLoading && !isFetching) {
      writeLiveWorkHint(false);
      setLiveWorkHint(false);
    }
  }, [feedbackZoneOpen, isLoading, isFetching]);

  // Orphan staging shells need re-upload — exclude from Retry Failed count.
  // SPEC-098 LAW-098-11: statusCounts.failed is pipeline-only (no delete_failed).
  const reprocessableFailedCount = useMemo(() => {
    const orphanReupload = (documents ?? []).filter(needsReuploadNotReprocess).length;
    return Math.max(0, (statusCounts.failed ?? 0) - orphanReupload);
  }, [documents, statusCounts.failed]);

  // Honest empty state while first ingest is in flight but list is still empty.
  const isBusyUpdating =
    documents.length === 0 &&
    (showFeedbackZone ||
      isUploading ||
      (pipelineStatus?.running_tasks ?? 0) > 0 ||
      (pipelineStatus?.queued_tasks ?? 0) > 0);

  // Debounced AT announcement: Deleting / Cleaning → Queued → live stages.
  const feedbackAnnouncement = useMemo(() => {
    const deleting = deleteSessions[0];
    if (deleting) {
      return `Deleting: ${deleting.documentName}${
        deleting.phaseLabel ? ` — ${deleting.phaseLabel}` : ''
      }`;
    }
    const admissionEntry = sessionReprocessEntries.find((e) =>
      shouldShowReprocessQueuingPanel(e.trackId),
    );
    if (admissionEntry) {
      const live = documents.find((d) => d.id === admissionEntry.documentId);
      const stage = (live?.current_stage || 'cleaning').toLowerCase();
      if (stage === 'cleaning') {
        return `Cleaning: ${admissionEntry.documentName}`;
      }
      if (stage === 'queued') {
        return `Queued: ${admissionEntry.documentName}`;
      }
      return `${admissionEntry.documentName}: ${stageDisplayName(stage)}`;
    }
    const primary = activeRunsDisplayed[0];
    if (primary) {
      if (primary.stage === 'cleaning') {
        return `Cleaning: ${primary.filename}`;
      }
      if (primary.stage === 'queued') {
        return `Queued: ${primary.filename}`;
      }
      const pct =
        primary.progress01 != null
          ? Math.round(primary.progress01 * 100)
          : undefined;
      const stage = stageDisplayName(String(primary.stage));
      return pct != null
        ? `${primary.filename}: ${stage}, ${pct}%`
        : `${primary.filename}: ${stage}`;
    }
    return '';
  }, [deleteSessions, sessionReprocessEntries, activeRunsDisplayed, documents]);

  // WHY the auto-seed useEffect was removed:
  //
  // The previous implementation called addReprocessEntry for every active run so
  // ProgressPanelRow would render after a page refresh. However, each
  // row polls GET /ingestion/{trackId}/progress every 5s.
  // That handler calls load_scoped_document_metadata — a full PostgreSQL scan of
  // ALL document metadata in the workspace. With N active documents, N scans fire
  // every 5s in addition to all other processing queries. Result: connection pool
  // exhaustion, health checks timing out (10+ s), cascade 500s on tenant endpoints.
  //
  // ActiveRunsPanel already shows all active runs from the documents cache with
  // zero extra DB queries beyond the existing 2s document-list poll. That is
  // sufficient feedback for background/post-refresh processing.
  //
  // ProgressPanelRow is now reserved for documents explicitly reprocessed in
  // the current session (addReprocessEntry is called from reprocessMutation and
  // bulk reprocess — typically 1–3 docs, dismissed by the user on completion).

  // OODA-16: Bulk selection extracted to useBulkSelection hook
  // SPEC-050 GAP-FIX: Bulk delete confirmation callback.
  // WHY: useBulkSelection owns selection state; DocumentManager owns the
  // confirmation dialog. The callback bridges them (SRP + DIP).
  // Defined before useBulkSelection so it can be passed as onDeleteRequested.
  const handleBulkDeleteRequested = useCallback((selectedDocuments: Document[]) => {
    setBulkDeleteTargets(selectedDocuments);
    setBulkDeleteDialogOpen(true);
  }, []);

  const {
    selectedIds,
    selectedCount,
    isAllSelected,
    handleSelectAll,
    handleSelectOne,
    handleClearSelection,
    handleBulkDelete,
    handleBulkReprocess,
    isBulkReprocessing,
  } = useBulkSelection({
    documents,
    onDeleteRequested: handleBulkDeleteRequested,
    // SPEC-051 GAP-051-02: wire bulk reprocess through ProgressPanelRow.
    // WHY: Previously bulk reprocess called reprocessDocument() directly and
    // discarded the track_id — no progress panel appeared. Now each reprocessed
    // document gets the same ProgressPanelRow as a single-doc reprocess.
    onReprocessTriggered: (name, trackId, opts) =>
      addReprocessEntry(name, trackId, opts),
    onReprocessDismissed: (documentId) =>
      removeReprocessEntryByDocumentId(documentId),
  });

  // SPEC-084 / GH-317: one durable batch-delete admit (not N× single deletes).
  // SPEC-098: paint-first sessions + optimistic deleting before HTTP returns.
  const handleBulkDeleteConfirmed = useCallback(async () => {
    const targets = [...bulkDeleteTargets];
    setBulkDeleteTargets([]);
    setBulkDeleteDialogOpen(false);
    handleClearSelection();
    if (targets.length === 0) return;
    const ids = targets.map((d) => d.id);
    for (const doc of targets) {
      beginDeleteSession({
        documentId: doc.id,
        documentName: doc.title || doc.file_name || doc.id,
      });
    }
    patchDocumentsDeletingOptimistic(queryClient, ids);
    try {
      const { batchDeleteDocuments } = await import(
        "@/lib/api/edgequake/documents"
      );
      const result = await batchDeleteDocuments(ids);
      for (const id of ids) {
        bindDeleteSessionTrackId(id, result.batch_track_id);
      }
      toast.success(
        t(
          "documents.bulk.deleteQueued",
          "Queued deletion of {{count}} document(s)",
          { count: result.planned_delete_count },
        ),
      );
    } catch (err) {
      const description =
        err instanceof Error ? err.message : t("common.unknownError", "Unknown error");
      for (const id of ids) {
        dismissDeleteSession(id);
      }
      toast.error(
        t("documents.bulk.deleteFailed", "Failed to queue bulk deletion"),
        { description },
      );
    }
  }, [bulkDeleteTargets, handleClearSelection, queryClient, t]);

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

  // OODA-22 / SPEC-048 DEF-06: Working vs Queued in tab title
  useDocumentTitle({
    totalCount,
    processingCount: pipelineUi.activeDocCount,
    queuedCount: pipelineUi.waitingDocCount,
  });

  const documentsActions = useMemo(
    () => ({
      onClick: handleDocumentClick,
      onDoubleClick: handleDocumentDoubleClick,
      onSelect: handleSelectOne,
      onReprocess: (doc: Document) => {
        if (needsReuploadNotReprocess(doc)) return;
        setReprocessTarget(doc);
      },
      onRetry: (doc: Document) => {
        if (needsReuploadNotReprocess(doc)) return;
        const name = doc.file_name || doc.title || doc.id.slice(0, 8);
        reprocessMutation.mutate({
          id: doc.id,
          name,
          isPdf: doc.source_type === 'pdf',
        });
      },
      onDelete: (doc: Document) => setDeleteConfirmTarget(doc),
      onViewDetails: handleViewDetails,
      onViewInGraph: handleViewInGraph,
      onViewPdf: handleViewPdf,
    }),
    [
      handleDocumentClick,
      handleDocumentDoubleClick,
      handleSelectOne,
      handleViewDetails,
      handleViewInGraph,
      handleViewPdf,
      reprocessMutation,
    ],
  );

  if (isError) {
    return <DocumentErrorAlert error={error} onRetry={refetch} />;
  }

  return (
    <DocumentsActionsProvider value={documentsActions}>
    <div
      className="flex h-full min-h-0 min-w-0 flex-1 overflow-clip"
      data-testid="documents-page-shell"
    >
      {/* Main Content - Flex column for proper scroll zones */}
      <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-clip">
        {/* Fixed Header Zone — title, filters, always-on dropzone stay pinned.
            max-h keeps inventory usable on short viewports (EC-099-01).
            SPEC-146: grow when SecurityFields expanded so selects are not clipped. */}
        <div
          className={`min-h-0 shrink-0 ${documentsChromeMaxClass(securityExpanded && docAbacEnabled)} space-y-3 overflow-y-auto overscroll-contain bg-background px-4 pt-4`}
          data-testid="documents-chrome"
        >
          <DocumentHeader
            totalCount={totalCount}
            countLabel={inventory.countLabel}
            failedCount={reprocessableFailedCount}
            showPipelineIndicator={pipelineUi.showPipelineIndicator}
            reservePipelineSlot={showFeedbackZone}
            pipelineAlertMode={pipelineUi.alertMode}
            activeDocCount={pipelineUi.activeDocCount}
            waitingDocCount={pipelineUi.waitingDocCount}
            pipelineWaitingOnly={pipelineUi.isQueuedOnly}
            pipelineDialogOpen={pipelineDialogOpen}
            onPipelineDialogChange={setPipelineDialogOpen}
            onRefresh={refetch}
            tenantId={selectedTenantId ?? undefined}
            workspaceId={selectedWorkspaceId ?? undefined}
            documents={documents}
            columnsMenu={
              <DropdownMenuCheckboxItem
                checked={showCostColumn}
                onCheckedChange={(v) => setShowCostColumn(Boolean(v))}
                data-testid="spec099-toggle-cost-column"
              >
                Show Cost column
              </DropdownMenuCheckboxItem>
            }
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
            classificationFilter={classificationFilter}
            onClassificationFilterChange={setClassificationFilter}
            showClassificationFilter={docAbacEnabled}
            pipelineStatus={pipelineStatus}
            pipelineUi={pipelineUi}
            documents={documents}
            onOpenPipelineDetails={() => setPipelineDialogOpen(true)}
            onReprocessStuckDocuments={(stuckDocs) => {
              for (const doc of stuckDocs) {
                // Staging orphans must be dismissed + re-uploaded, not reprocessed.
                if (needsReuploadNotReprocess(doc)) continue;
                const name =
                  doc.file_name?.trim() ||
                  doc.title?.trim() ||
                  doc.id.slice(0, 8);
                const isPdf =
                  doc.source_type === 'pdf' ||
                  Boolean(doc.pdf_id) ||
                  /\.pdf$/i.test(name);
                reprocessMutation.mutate({
                  id: doc.id,
                  mode: 'full',
                  name,
                  isPdf,
                });
              }
            }}
            isReprocessingStuck={reprocessMutation.isPending}
            demotePipelineBanner={showFeedbackZone}
            collapseUploadSlot={showFeedbackZone}
            getRootProps={getRootProps}
            getInputProps={getInputProps}
            isDragActive={isDragActive}
            openFileDialog={openFileDialog}
            pdfParserBackend={pdfParserBackend}
            onPdfParserBackendChange={setPdfParserBackend}
            workspacePdfParserBackend={selectedWorkspace?.pdf_parser_backend}
            visionReasoningEffort={visionReasoningEffort}
            onVisionReasoningEffortChange={setVisionReasoningEffort}
            visionExtract={visionExtract}
            onVisionExtractChange={setVisionExtract}
            visionProvider={
              selectedWorkspace?.vision_llm_provider ??
              selectedWorkspace?.llm_provider
            }
            visionModel={
              selectedWorkspace?.vision_llm_model ?? selectedWorkspace?.llm_model
            }
            securityFields={securityFields}
            onSecurityFieldsChange={setSecurityFields}
            showSecurityFields={docAbacEnabled}
            onSecurityExpandedChange={handleSecurityExpandedChange}
            selectedCount={selectedCount}
            onBulkReprocess={() => {
              // WHY: Open the bulk choice dialog so the user picks full
              // re-conversion vs. entity-only before reprocessing the batch.
              if (selectedCount === 0) return;
              setBulkReprocessOpen(true);
            }}
            onBulkDelete={handleBulkDelete}
            onClearSelection={handleClearSelection}
          />

        </div>

      {/* ─── Unified feedback zone ───────────────────────────────────────────
          WHY one zone instead of two separate capped sections:
          Previously ActiveRunsPanel was capped at 28 vh inside the toolbar and
          reprocess panels were capped at 30 vh below it → combined worst-case
          58 vh + toolbar ≈ 500 px, leaving the table with <50 px (1 row visible).

          Now ALL variable-height feedback (active-run stepper, upload progress,
          reprocess panels) shares a SINGLE 35 vh cap and a single scroll boundary.
          Layout guarantee:
            static toolbar  ≈ 150 px  (search + filters + banner + dropzone + batch)
            feedback zone   ≤ 35 vh   (scrolls internally when full)
            table           = flex-1  (always gets the remaining ≥65 vh − 150 px)
          On 760 px viewport: table ≥ 760×0.65−150 ≈ 344 px → ~5 rows always visible.
      ─────────────────────────────────────────────────────────────────────── */}
      {showFeedbackZone && (
        <ApiErrorBoundary
          fallback={() => (
            <div
              role="alert"
              className="shrink-0 border-b px-4 py-2 text-sm text-muted-foreground"
              data-testid="spec051-feedback-zone-fallback"
            >
              Progress unavailable — processing continues in the background.
            </div>
          )}
        >
        <div
          className="shrink-0 overflow-y-auto border-b bg-background"
          style={{
            maxHeight: '35vh',
            // CLS floor only while skeleton reservation is active. Live content
            // sizes naturally; ordinary Failed must not hold an empty 208px band.
            ...(reserveFeedbackSlot
              ? { minHeight: FEEDBACK_ZONE_RESERVE_MIN_PX }
              : {}),
          }}
          data-testid="spec051-feedback-zone"
          data-reserved={reserveFeedbackSlot ? 'true' : 'false'}
          aria-labelledby="spec051-feedback-zone-label"
        >
          <span id="spec051-feedback-zone-label" className="sr-only">
            Document processing progress
          </span>
          {reserveFeedbackSlot ? (
            <div className="px-4 py-2">
              <FeedbackZoneSkeleton />
            </div>
          ) : (
            <>
          <FeedbackZoneLiveRegion announcement={feedbackAnnouncement} />
          <div className="px-4 py-2 space-y-2">
            {/* Server-stage stepper — includes stuck docs (per-doc cards stay visible) */}
            {showActiveRuns && (
              <ActiveRunsPanel
                runs={activeRunsDisplayed}
                onDismissFailed={handleDeleteDocument}
              />
            )}

            {/* Upload progress: client-only rows always; tracked rows when
                ActiveRunsPanel is hidden (it handles them when visible). */}
            {showUploadList && (
              <UploadProgressList
                uploadingFiles={uploadFilesForList}
                isUploading={isUploading}
                onRemove={removeUploadingFile}
                onComplete={handleUploadComplete}
                onFailed={handleUploadFailed}
                embedded
              />
            )}

            {/* Per-document reprocess progress panels */}
            {sessionReprocessEntries.length > 0 && (
              <div data-testid="spec051-reprocess-progress-panels">
                <h4 className="text-sm font-semibold flex items-center gap-2 text-muted-foreground mb-1.5">
                  <span className="h-2 w-2 rounded-full bg-sky-500 animate-pulse" />
                  {t('documents.reprocess.progressHeader', 'Reprocessing {{count}} document(s)', {
                    count: sessionReprocessEntries.length,
                  })}
                </h4>
                <div className="space-y-1.5">
                  {sessionReprocessEntries.map((entry) => {
                    const liveDoc = documents.find((d) => d.id === entry.documentId);
                    // Keep Queuing on provisional entry; never poll batch reprocess_*.
                    // Prefer server track only when it is a live task progress key.
                    const liveTrackId = resolveReprocessPanelTrackId(
                      entry.trackId,
                      liveDoc?.track_id,
                    );
                    const unpinRow = () => {
                      unpinReprocessDocuments(entry.documentId);
                    };
                    // Dismiss/cancel: immediate remove + suppress bind re-add.
                    const dismissSessionPanel = () => {
                      unpinRow();
                      removeReprocessEntryByDocumentId(entry.documentId);
                    };
                    // Terminal: brief visibility, then delayed remove (upload parity).
                    const finishSessionPanel = () => {
                      unpinRow();
                      removeReprocessEntry(entry.trackId);
                    };
                    return (
                      <ProgressPanelRow
                        key={entry.documentId}
                        trackId={liveTrackId}
                        documentName={entry.documentName}
                        isPdf={shouldUsePdfReprocessPanel(Boolean(entry.isPdf), entry.mode)}
                        currentStage={liveDoc?.current_stage ?? 'cleaning'}
                        stageMessage={liveDoc?.stage_message}
                        onRemove={dismissSessionPanel}
                        onComplete={finishSessionPanel}
                        onFailed={finishSessionPanel}
                        onCancel={dismissSessionPanel}
                        data-testid="spec051-reprocess-panel"
                        data-track-id={liveTrackId}
                      />
                    );
                  })}
                </div>
              </div>
            )}

            {/* SPEC-050: Per-document delete progress (WS phases) */}
            {deleteSessions.length > 0 && (
              <div data-testid="spec050-delete-progress-panels">
                <h4
                  className="text-sm font-semibold flex items-center gap-2 text-muted-foreground mb-1.5"
                  data-testid="spec098-delete-progress-header"
                  data-active-count={deleteProgressHeader.activeCount}
                  data-failed-count={deleteProgressHeader.failedCount}
                >
                  <span
                    className={`h-2 w-2 rounded-full bg-rose-500${
                      deleteProgressHeader.pulse ? ' animate-pulse' : ''
                    }`}
                  />
                  {deleteProgressHeader.text}
                </h4>
                <div className="space-y-1.5">
                  {deleteSessions.map((entry) => {
                    const dismissHint = t(
                      'documents.delete.dismissHint',
                      'Hides progress; deletion continues.',
                    );
                    return (
                      <div
                        key={entry.documentId}
                        className="relative p-2 rounded-lg border bg-card"
                        data-testid="spec050-delete-panel"
                        data-document-id={entry.documentId}
                        data-phase={entry.phase ?? 'starting'}
                        data-status={entry.status}
                      >
                        <AdmissionPhaseRow
                          phase="deleting"
                          documentName={entry.documentName}
                          sessionStatus={entry.status}
                          stageMessage={
                            entry.status === 'failed'
                              ? entry.error ||
                                entry.phaseLabel ||
                                'Deletion failed — dismiss this panel'
                              : formatDeleteStageMessage(entry, deleteNow)
                          }
                          countsLabel={formatDeleteCountsLabel(entry)}
                          variant="row"
                          data-testid="delete-progress-row"
                        />
                        <Button
                          variant="ghost"
                          size="icon"
                          className="absolute top-1 right-1 h-8 w-8"
                          onClick={() => dismissDeleteSession(entry.documentId)}
                          aria-label={t(
                            'documents.delete.dismissAria',
                            'Dismiss progress — hides progress; deletion continues',
                          )}
                          title={dismissHint}
                        >
                          <X className="h-4 w-4" />
                          <span className="sr-only">{dismissHint}</span>
                        </Button>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}
          </div>
            </>
          )}
        </div>
        </ApiErrorBoundary>
      )}

      {/* OODA-26: Table section extracted to DocumentTableSection */}
      <DocumentTableSection
        documents={documents}
        totalCount={totalCount}
        isLoading={isInitialLoading}
        isBusyUpdating={isBusyUpdating}
        selectedIds={selectedIds}
        selectedDocument={selectedDocument}
        searchQuery={searchQuery}
        statusFilter={statusFilter}
        isAllSelected={isAllSelected}
        activeRunDocumentIds={workingRunDocumentIds}
        showCostColumn={showCostColumn}
        showAbacColumns={docAbacEnabled}
        overflowLabel={inventory.overflowLabel}
        onSelectAll={handleSelectAll}
        onSelectOne={handleSelectOne}
        onRowClick={handleDocumentClick}
        onRowDoubleClick={handleDocumentDoubleClick}
        onViewDetails={handleViewDetails}
        onViewInGraph={handleViewInGraph}
        onViewPdf={handleViewPdf}
        onRetry={(id) => {
          // Pass document name + isPdf for ProgressPanelRow display
          const doc = documents.find((d) => d.id === id);
          if (doc && needsReuploadNotReprocess(doc)) return;
          const name = doc?.file_name || doc?.title || id.slice(0, 8);
          reprocessMutation.mutate({ id, name, isPdf: doc?.source_type === 'pdf' });
        }}
        onRetryLabels={(doc) => {
          setRetryLabelsDoc(doc);
          setRetryLabelFields({
            ...DEFAULT_SECURITY_FIELDS,
            classification: doc.classification || 'internal',
            share_mode:
              (doc.share_mode as SecurityFields['share_mode']) || 'workspace',
            export_control: Boolean(doc.export_control),
            pii: Boolean(doc.pii),
            project_id: doc.project_id || undefined,
            security_status: 'quarantined',
          });
        }}
        onReprocess={(id) => {
          // WHY: Open the choice dialog for the target document so the user can
          // pick between full PDF re-conversion and entity-only re-extraction.
          const target = documents.find((d) => d.id === id) ?? null;
          if (target && needsReuploadNotReprocess(target)) return;
          setReprocessTarget(target ?? ({ id } as Document));
        }}
        onCancel={(trackId) => cancelMutation.mutate(trackId)}
        onDelete={handleDeleteDocument}
        isRetrying={reprocessMutation.isPending || retryingLabels}
        isCancelling={cancelMutation.isPending}
        deletingDocumentIds={deletingDocumentIds}
        onUploadClick={openFileDialog}
        onClearFilter={() => {
          setStatusFilter('all');
          setClassificationFilter('all');
          setSearchQuery('');
        }}
        sortField={sortField}
        sortDirection={sortDirection}
        onSort={handleColumnSort}
      />
      {totalCount > 0 && (
        <div
          className="shrink-0 border-t bg-background px-4"
          data-testid="documents-pagination"
        >
          <PaginationControls
            currentPage={currentPage}
            totalPages={totalPages}
            pageSize={pageSize}
            totalItems={totalCount}
            onPageChange={setCurrentPage}
            onPageSizeChange={setPageSize}
          />
        </div>
      )}
      </div>

      {/* OADA-27: Right panel extracted to DocumentPreviewRightPanel */}
      <DocumentPreviewRightPanel
        isOpen={previewPanelOpen}
        onToggle={() => setPreviewPanelOpen(!previewPanelOpen)}
        onClose={handlePreviewClose}
        selectedDocument={selectedDocument}
        onDelete={(id) => {
          // SPEC-050 GAP-FIX: Route preview panel delete through confirm dialog.
          // WHY: Previously called handleDeleteDocument directly — no impact preview.
          const target = documents.find((d) => d.id === id) ?? selectedDocument;
          if (target) {
            setDeleteConfirmTarget(target);
          } else {
            // Fallback: direct delete if we can't find the document
            handleDeleteDocument(id);
          }
        }}
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
          // SPEC-050-REPROCESS: Pass document name so ProgressPanelRow shows
          // a meaningful filename instead of a truncated ID.
          const docName =
            reprocessTarget.file_name ||
            reprocessTarget.title ||
            reprocessTarget.id.slice(0, 8);
          reprocessMutation.mutate({
            id: reprocessTarget.id,
            mode: choice.mode,
            name: docName,
            isPdf: reprocessTarget.source_type === 'pdf',
          });
          setReprocessTarget(null);
        }}
        onCancel={() => setReprocessTarget(null)}
      />

      {/* Bulk reprocess choice dialog — one mode applied to all selected docs. */}
      <BulkReprocessDialog
        open={bulkReprocessOpen}
        count={selectedCount}
        isBusy={isBulkReprocessing}
        onConfirm={(choice: BulkReprocessChoice) => {
          // Start first so sync provisional pin/panel paints before dialog close.
          void handleBulkReprocess(choice.mode);
          setBulkReprocessOpen(false);
        }}
        onCancel={() => {
          if (!isBulkReprocessing) setBulkReprocessOpen(false);
        }}
      />

      {/* SPEC-050 GAP-FIX: Bulk delete confirmation (toolbar Delete button).
          WHY: Previously the toolbar Delete fired deleteDocument() directly with
          no confirmation or impact preview. This dialog gives the same quality
          of experience as the per-row delete in DocumentActionsMenu. */}
      <BulkDeleteConfirmDialog
        open={bulkDeleteDialogOpen}
        onOpenChange={(open) => {
          setBulkDeleteDialogOpen(open);
          if (!open) setBulkDeleteTargets([]);
        }}
        documents={bulkDeleteTargets}
        onConfirm={handleBulkDeleteConfirmed}
        isDeleting={deleteMutation.isPending}
      />

      {/* SPEC-050 GAP-FIX: Single delete confirm for preview panel.
          WHY: The preview panel's Delete button previously called handleDeleteDocument()
          directly — bypassing the confirm dialog. Now it opens this dialog first. */}
      <DeleteConfirmDialog
        open={deleteConfirmTarget !== null}
        onOpenChange={(open) => {
          if (!open) setDeleteConfirmTarget(null);
        }}
        document={deleteConfirmTarget}
        onConfirm={(id) => {
          handleDeleteDocument(id);
          setDeleteConfirmTarget(null);
        }}
        isDeleting={deleteMutation.isPending}
      />

      {/* SPEC-146: quarantine recovery — edit labels then PATCH (never blind re-PATCH). */}
      <Dialog
        open={retryLabelsDoc !== null}
        onOpenChange={(open) => {
          if (!open) setRetryLabelsDoc(null);
        }}
      >
        <DialogContent data-testid="spec146-retry-labels-dialog">
          <DialogHeader>
            <DialogTitle>
              {t('security.retryLabels', 'Retry labels')}
            </DialogTitle>
          </DialogHeader>
          <SecurityFieldsForm
            value={retryLabelFields}
            onChange={setRetryLabelFields}
            alwaysExpanded
          />
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setRetryLabelsDoc(null)}
            >
              {t('common.cancel', 'Cancel')}
            </Button>
            <Button
              type="button"
              data-testid="spec146-retry-labels-submit"
              disabled={retryingLabels || !retryLabelsDoc}
              onClick={async () => {
                if (!retryLabelsDoc) return;
                setRetryingLabels(true);
                try {
                  await patchDocumentSecurityLabels(
                    retryLabelsDoc.id,
                    retryLabelFields,
                  );
                  toast.success(t('security.retryLabels', 'Retry labels'));
                  setRetryLabelsDoc(null);
                  await refetch();
                } catch (e) {
                  toast.error(
                    e instanceof Error ? e.message : 'Retry labels failed',
                  );
                } finally {
                  setRetryingLabels(false);
                }
              }}
            >
              {t('security.saveLabels', 'Save labels')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
    </DocumentsActionsProvider>
  );
}

export default DocumentManager;
