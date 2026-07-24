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

import { useRouter } from 'next/navigation';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

import { nextDocumentSortState } from '@/lib/documents/document-sort';
import {
  beginDeleteSession,
  dismissDeleteSession,
  formatDeleteCountsLabel,
  formatDeleteStageMessage,
  patchDocumentsDeletingOptimistic,
} from '@/lib/documents/deletion-session';
import {
  documentIdsWithQueuingSession,
  filterRunsExcludingQueuingSession,
  resolveReprocessPanelTrackId,
  shouldShowReprocessQueuingPanel,
  unpinReprocessDocuments,
} from '@/lib/documents/progress-admit';
import {
  buildIngestionRunViews,
  stageDisplayName,
} from '@/lib/pipeline/ingestion-run-view';
import {
  hasQueueCoverage,
  needsReuploadNotReprocess,
  resolvePipelineUiState,
} from '@/lib/pipeline/pipeline-document-state';

import { useBulkSelection } from '@/hooks/use-bulk-selection';
import { useDeletionSessions } from '@/hooks/use-deletion-progress';
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
import { DocumentTableSection } from './document-table-section';
import { DocumentToolbarSection } from './document-toolbar-section';
import { DuplicateUploadDialog } from './duplicate-upload-dialog';
import { FeedbackZoneLiveRegion } from './feedback-zone-live-region';
import { LargePdfAdmissionDialog } from './large-pdf-admission-dialog';
import { ProgressPanelRow } from './progress-panel-row';
import { ReprocessDialog, type ReprocessChoice } from './reprocess-dialog';
import { ApiErrorBoundary } from '@/components/shared/api-error-boundary';
import { Button } from '@/components/ui/button';
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
  const [pdfParserBackend, setPdfParserBackend] = useState<'default' | 'vision' | 'edgeparse'>('default');
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

  // VS-03: No pagination state — virtual scrolling handles windowing client-side.
  // We fetch all documents at once (up to VIRTUAL_PAGE_SIZE) and let the
  // virtualizer render only visible rows. This eliminates pagination UI entirely.

  // OODA-17: Filter/sort preferences with localStorage persistence
  const {
    statusFilter, setStatusFilter,
    sortField, setSortField,
    sortDirection, setSortDirection,
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

  // SPEC-050: Track which document IDs are currently being deleted so rows can
  // show "Deleting" visual state immediately on confirm (before query invalidation).
  const [deletingDocumentIds, setDeletingDocumentIds] = useState<Set<string>>(new Set());

  // Feedback-zone delete sessions (WS phase updates).
  const deleteSessions = useDeletionSessions();

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

  // OODA-29: Document queries extracted to useDocumentQueries hook
  // VS-03: page=1 with large pageSize fetches everything at once for virtual scroll
  // SPEC-084 / GH-319: match API MAX_PAGE_SIZE (budget clamp). Larger values
  // were silently truncated to 100 while the UI assumed a full fetch.
  const VIRTUAL_PAGE_SIZE = 100;
  const { data, isLoading, isError, error, refetch, pipelineStatus, queryClient } = useDocumentQueries({
    tenantId: selectedTenantId,
    workspaceId: selectedWorkspaceId,
    currentPage: 1,
    pageSize: VIRTUAL_PAGE_SIZE,
    statusFilter,
  });

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
      setDeletingDocumentIds((prev) => new Set([...prev, id]));
      deleteMutation.mutate(id, {
        onSettled: () => {
          setDeletingDocumentIds((prev) => {
            const next = new Set(prev);
            next.delete(id);
            return next;
          });
        },
      });
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

  // SPEC-048: clear upload chrome when documents reach terminal state
  useEffect(() => {
    pruneTerminalUploads(documents ?? []);
    // SPEC-050-REPROCESS: also prune reprocess progress panels on terminal state
    pruneTerminalReprocessEntries(documents ?? []);
  }, [documents, pruneTerminalUploads, pruneTerminalReprocessEntries]);

  const pipelineUi = useMemo(
    () => resolvePipelineUiState(documents, pipelineStatus),
    [documents, pipelineStatus],
  );

  const runViewOpts = useMemo(() => {
    const pending =
      pipelineStatus?.pending_tasks ?? pipelineStatus?.queued_tasks ?? 0;
    const processing =
      pipelineStatus?.processing_tasks ?? pipelineStatus?.running_tasks ?? 0;
    return {
      hasQueueCoverage: hasQueueCoverage(pipelineStatus, pending, processing),
    };
  }, [pipelineStatus]);

  // Orphan staging shells need re-upload — exclude from Retry Failed count.
  const reprocessableFailedCount = useMemo(() => {
    const orphanReupload = (documents ?? []).filter(needsReuploadNotReprocess).length;
    return Math.max(0, (statusCounts.failed ?? 0) - orphanReupload);
  }, [documents, statusCounts.failed]);

  // Only mute siblings while a run is actively working (not merely queued)
  const workingRunDocumentIds = useMemo(() => {
    const ids = new Set<string>();
    for (const run of buildIngestionRunViews(documents, runViewOpts).values()) {
      if (run.stageStatus === 'active') ids.add(run.documentId);
    }
    return ids;
  }, [documents, runViewOpts]);

  // All active / pending runs — used for the unified feedback zone.
  // WHY compute once: buildIngestionRunViews was called separately for
  // workingRunDocumentIds and activeRunViews; this avoids the duplication.
  const allRuns = useMemo(
    () => [...buildIngestionRunViews(documents, runViewOpts).values()],
    [documents, runViewOpts],
  );

  // While a session panel is still provisional Queuing (not cleaning), hide the
  // ActiveRuns card for that documentId. Cleaning keeps ActiveRuns so the
  // stepper narrates graph cleanup alongside the session AdmissionPhaseRow.
  const stagesByDocId = useMemo(() => {
    const map = new Map<string, string | null | undefined>();
    for (const doc of documents ?? []) {
      map.set(doc.id, doc.current_stage);
    }
    return map;
  }, [documents]);
  const queuingSessionDocIds = useMemo(
    () => documentIdsWithQueuingSession(reprocessEntries, stagesByDocId),
    [reprocessEntries, stagesByDocId],
  );
  const activeRunsForPanel = useMemo(
    () => filterRunsExcludingQueuingSession(allRuns, queuingSessionDocIds),
    [allRuns, queuingSessionDocIds],
  );

  // Unified feedback zone: keep ActiveRuns even when stuck so per-doc cards
  // remain visible; stuck toolbar banner still owns the recover CTA.
  const showActiveRuns = activeRunsForPanel.length > 0;

  // Prefer stuck docs in the zone when alertMode is stuck (more relevant).
  const activeRunsDisplayed = useMemo(() => {
    if (pipelineUi.alertMode !== 'stuck' || pipelineUi.stuckDocs.length === 0) {
      return activeRunsForPanel;
    }
    const stuckIds = new Set(pipelineUi.stuckDocs.map((d) => d.id));
    const stuckRuns = activeRunsForPanel.filter((r) => stuckIds.has(r.documentId));
    return stuckRuns.length > 0 ? stuckRuns : activeRunsForPanel;
  }, [activeRunsForPanel, pipelineUi.alertMode, pipelineUi.stuckDocs]);

  // Session ProgressPanelRow: always show Queuing; keep PDF-full phase panels;
  // hand off entities/merge only after ActiveRuns is actually painted for that doc.
  const sessionReprocessEntries = useMemo(
    () =>
      reprocessEntries.filter((entry) => {
        if (shouldShowReprocessQueuingPanel(entry.trackId)) return true;
        if (shouldUsePdfReprocessPanel(entry.isPdf, entry.mode)) return true;
        // Avoid empty gap: keep session row until ActiveRuns shows this documentId.
        if (!showActiveRuns) return true;
        return !activeRunsDisplayed.some((r) => r.documentId === entry.documentId);
      }),
    [reprocessEntries, showActiveRuns, activeRunsDisplayed],
  );

  // All active / pending runs (kept for potential future use).
  // NOTE: not used to auto-seed reprocessEntries — see WHY below.
  const activeRunViews = useMemo(
    () => allRuns.filter((r) => r.stageStatus === 'active' || r.stageStatus === 'pending'),
    [allRuns],
  );

  // Upload list split: client-only rows show always; tracked rows only when
  // ActiveRunsPanel is hidden (it handles them visually when active).
  const clientOnlyUploads = useMemo(
    () => uploadingFiles.filter((f) => !f.trackId),
    [uploadingFiles],
  );
  const trackedUploads = useMemo(
    () => uploadingFiles.filter((f) => Boolean(f.trackId)),
    [uploadingFiles],
  );
  const showUploadList =
    clientOnlyUploads.length > 0 || (trackedUploads.length > 0 && !showActiveRuns);
  const uploadFilesForList = showActiveRuns ? clientOnlyUploads : uploadingFiles;

  const feedbackZoneOpen =
    showActiveRuns ||
    showUploadList ||
    sessionReprocessEntries.length > 0 ||
    deleteSessions.length > 0;

  // Honest empty state while first ingest is in flight but list is still empty.
  const isBusyUpdating =
    documents.length === 0 &&
    (feedbackZoneOpen ||
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
  const handleBulkDeleteConfirmed = useCallback(async () => {
    const targets = [...bulkDeleteTargets];
    setBulkDeleteTargets([]);
    setBulkDeleteDialogOpen(false);
    handleClearSelection();
    if (targets.length === 0) return;
    try {
      const { batchDeleteDocuments } = await import(
        "@/lib/api/edgequake/documents"
      );
      const result = await batchDeleteDocuments(targets.map((d) => d.id));
      for (const doc of targets) {
        beginDeleteSession({
          documentId: doc.id,
          documentName: doc.title || doc.file_name || doc.id,
          trackId: result.batch_track_id,
        });
      }
      patchDocumentsDeletingOptimistic(
        queryClient,
        targets.map((d) => d.id),
      );
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
            failedCount={reprocessableFailedCount}
            showPipelineIndicator={pipelineUi.showPipelineIndicator}
            pipelineAlertMode={pipelineUi.alertMode}
            activeDocCount={pipelineUi.activeDocCount}
            pipelineWaitingOnly={pipelineUi.isQueuedOnly}
            pipelineDialogOpen={pipelineDialogOpen}
            onPipelineDialogChange={setPipelineDialogOpen}
            onRefresh={refetch}
            tenantId={selectedTenantId ?? undefined}
            workspaceId={selectedWorkspaceId ?? undefined}
            documents={documents}
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
            demotePipelineBanner={feedbackZoneOpen}
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
      {feedbackZoneOpen && (
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
          style={{ maxHeight: '35vh' }}
          data-testid="spec051-feedback-zone"
          aria-labelledby="spec051-feedback-zone-label"
        >
          <span id="spec051-feedback-zone-label" className="sr-only">
            Document processing progress
          </span>
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
                        isPdf={shouldUsePdfReprocessPanel(entry.isPdf, entry.mode)}
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
                <h4 className="text-sm font-semibold flex items-center gap-2 text-muted-foreground mb-1.5">
                  <span className="h-2 w-2 rounded-full bg-rose-500 animate-pulse" />
                  {t('documents.delete.progressHeader', 'Deleting {{count}} document(s)', {
                    count: deleteSessions.length,
                  })}
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
        </div>
        </ApiErrorBoundary>
      )}

      {/* OODA-26: Table section extracted to DocumentTableSection */}
      <DocumentTableSection
        documents={documents}
        totalCount={totalCount}
        isLoading={isLoading}
        isBusyUpdating={isBusyUpdating}
        selectedIds={selectedIds}
        selectedDocument={selectedDocument}
        searchQuery={searchQuery}
        statusFilter={statusFilter}
        isAllSelected={isAllSelected}
        activeRunDocumentIds={workingRunDocumentIds}
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
        onReprocess={(id) => {
          // WHY: Open the choice dialog for the target document so the user can
          // pick between full PDF re-conversion and entity-only re-extraction.
          const target = documents.find((d) => d.id === id) ?? null;
          if (target && needsReuploadNotReprocess(target)) return;
          setReprocessTarget(target ?? ({ id } as Document));
        }}
        onCancel={(trackId) => cancelMutation.mutate(trackId)}
        onDelete={handleDeleteDocument}
        isRetrying={reprocessMutation.isPending}
        isCancelling={cancelMutation.isPending}
        deletingDocumentIds={deletingDocumentIds}
        onUploadClick={openFileDialog}
        onClearFilter={() => {
          setStatusFilter('all');
          setSearchQuery('');
        }}
        sortField={sortField}
        sortDirection={sortDirection}
        onSort={handleColumnSort}
      />
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
    </div>
  );
}

export default DocumentManager;
