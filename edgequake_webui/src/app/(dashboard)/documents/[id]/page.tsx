'use client';

import { ContentRenderer } from '@/components/document/content-renderer';
import {
  DocumentImageViewer,
  isImageDocument,
} from '@/components/documents/document-image-viewer';
import { MetadataSidebar } from '@/components/document/metadata-sidebar';
import { DetailLifecycleBadge } from '@/components/documents/detail-lifecycle-badge';
import { DocumentDownloadMenu } from '@/components/documents/document-download-menu';
import { ProgressPanelRow } from '@/components/documents/progress-panel-row';
import { PDFViewer } from '@/components/documents/pdf-viewer';
import {
    ReprocessDialog,
    type ReprocessChoice,
} from '@/components/documents/reprocess-dialog';
import { ReprocessPagesDialog } from '@/components/documents/reprocess-pages-dialog';
import { PageHealthStrip } from '@/components/documents/page-health-strip';
import { PageSyncModeControl } from '@/components/documents/page-sync-mode-control';
import { usePageHealth } from '@/hooks/use-page-health';
import { resolveDetailLifecycle } from '@/lib/documents/detail-lifecycle';
import { pdfCurrentPageForMode } from '@/lib/documents/page-sync-mode';
import { SideBySideViewer } from '@/components/documents/side-by-side-viewer';
import { Button } from '@/components/ui/button';
import { ResizablePanel } from '@/components/ui/resizable-panel';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { usePageSyncController } from '@/hooks/use-page-sync-controller';
import { useMediaQuery } from '@/hooks/use-media-query';
import { useQueryHandoff } from '@/hooks/use-query-handoff';
import { shouldUsePdfReprocessPanel } from '@/hooks/use-reprocess-tracking';
import {
    getDocument,
    getPdfContent,
    getPdfDownloadUrl,
    includeDocumentAssetsFromPdf,
    reprocessDocument,
} from '@/lib/api/edgequake';
import { cancelDocumentRun } from '@/lib/documents/cancel-document-run';
import {
  abortAdmit,
  admitQueuingToastId,
  beginAdmit,
  bindLiveTask,
  resolveReprocessPanelTrackId,
} from '@/lib/documents/progress-admit';
import { getEffectiveErrorMessage } from '@/lib/utils/document-status';
import { parsePageParam } from '@/lib/utils/document-url';
import { resolvePdfId, withPdfMarkdown } from '@/lib/documents/viewer-target';
import { extractPageMarkdown, hasPageMarkers } from '@/lib/utils/page-markers';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
    AlertCircle,
    ArrowLeft,
    ChevronLeft,
    ChevronRight,
    FileText,
    Loader2,
    MessageSquareText,
    Network,
    RefreshCw,
    RotateCcw,
    StopCircle,
} from 'lucide-react';
import Link from 'next/link';
import { useParams, useRouter, useSearchParams } from 'next/navigation';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

export default function DocumentViewPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const params = useParams();
  const searchParams = useSearchParams();
  const documentId = params.id as string;
  const isLgUp = useMediaQuery('(min-width: 1024px)');
  const { selectedWorkspaceId } = useTenantStore();
  const queryClient = useQueryClient();

  // SPEC-051 GAP-051-01: Reprocess dialog + progress panel state for the detail page.
  // WHY: Previously the detail page had NO reprocess action for failed/cancelled docs —
  // the cancelled message literally told users to go back to the list.
  const [reprocessDialogOpen, setReprocessDialogOpen] = useState(false);
  const [pagesReprocessOpen, setPagesReprocessOpen] = useState(false);
  /** Seed selection when opening the reprocess-pages modal. */
  const [selectedPages, setSelectedPages] = useState<number[]>([]);
  /** Pages marked running optimistically until /pages/health catches up. */
  const [pendingReprocessPages, setPendingReprocessPages] = useState<number[]>(
    [],
  );
  // Progress SSOT: server task_id (not batch reprocess_*). GAP-051-03.
  const [reprocessTrackId, setReprocessTrackId] = useState<string | null>(null);
  const [reprocessMode, setReprocessMode] = useState<'entities' | 'full'>('entities');

  // SPEC-051: Reprocess mutation for the detail page — same admit lifecycle as list.
  const reprocessMutationDetail = useMutation({
    mutationFn: ({ mode }: { mode: 'entities' | 'full' }) =>
      reprocessDocument(documentId, true, mode),
    onMutate: async ({ mode }) => {
      const previousDocuments = queryClient.getQueriesData({
        queryKey: ['documents'],
      });
      const provisionalByDoc = beginAdmit(queryClient, documentId);
      const provisional = provisionalByDoc.get(documentId);
      if (provisional) {
        setReprocessMode(mode);
        setReprocessTrackId(provisional);
      }
      toast.loading(t('documents.reprocess.queuing', 'Queuing reprocess…'), {
        id: admitQueuingToastId(documentId),
      });
      await queryClient.cancelQueries({ queryKey: ['documents'] });
      return { previousDocuments };
    },
    onSuccess: (data, { mode }) => {
      toast.dismiss(admitQueuingToastId(documentId));
      const progressTrackId = bindLiveTask(queryClient, documentId, data);
      setReprocessMode(mode);

      if (!progressTrackId) {
        abortAdmit(queryClient, documentId);
        setReprocessTrackId(null);
        const reasons = data.skip_reasons
          ? Object.entries(data.skip_reasons)
              .map(([reason, count]) => `${reason} (${count})`)
              .join(', ')
          : '';
        toast.warning(
          t('documents.reprocess.skipped', 'Document was not requeued for processing'),
          {
            description:
              reasons ||
              t(
                'documents.reprocess.skippedHint',
                'It may already be processing, or content is missing.',
              ),
            duration: 6000,
          },
        );
        queryClient.invalidateQueries({ queryKey: ['document', documentId] });
        return;
      }

      setReprocessTrackId(progressTrackId);
      toast.success(
        t('documents.reprocess.success', 'Document queued for reprocessing'),
        { duration: 4000 },
      );
      setTimeout(() => {
        queryClient.invalidateQueries({ queryKey: ['document', documentId] });
      }, 2000);
    },
    onError: (error: Error, _vars, context) => {
      toast.dismiss(admitQueuingToastId(documentId));
      abortAdmit(queryClient, documentId, context?.previousDocuments);
      setReprocessTrackId(null);
      toast.error(
        t('documents.reprocess.failed', 'Reprocess failed'),
        { description: error.message },
      );
    },
  });

  // SPEC-051: Cancel mutation for the detail page.
  const cancelMutationDetail = useMutation({
    // SPEC-155: document-keyed — an orphan row without a track_id still cancels.
    mutationFn: async (doc: { id: string; track_id?: string | null }) => {
      const outcome = await cancelDocumentRun(queryClient, {
        documentId: doc.id,
        trackId: doc.track_id,
      });
      if (outcome === 'failed') throw new Error('Cancel failed');
    },
    onSuccess: () => {
      toast.success(
        t('documents.cancel.success', 'Document processing cancelled'),
        { duration: 4000 },
      );
      queryClient.invalidateQueries({ queryKey: ['document', documentId] });
    },
    onError: (error: Error) => {
      toast.error(
        t('documents.cancel.failed', 'Cancel failed'),
        { description: error.message },
      );
    },
  });
  
  // Get highlight parameters from URL
  const highlightText = searchParams.get('highlight') || undefined;
  const startLine = searchParams.get('start_line') 
    ? parseInt(searchParams.get('start_line')!) 
    : undefined;
  const endLine = searchParams.get('end_line') 
    ? parseInt(searchParams.get('end_line')!) 
    : undefined;
  // Deep-link: chunk UUID passed from query citation click
  const chunkIdFromUrl = searchParams.get('chunk') || undefined;

  // SPEC-032 W-09: PDF page deep-link — ?page=N or #page=N from citation "Go to page"
  // Priority: URL search param `page=N` > hash fragment `#page=N` > default 1
  // The URL param is set by source-citations.tsx in the deep-link href.
  const pageFromUrl = useMemo(
    () => parsePageParam(searchParams.get('page')),
    [searchParams],
  );

  const [resolvedPdfPage, setResolvedPdfPage] = useState<number | undefined>();
  const activePdfPage = pageFromUrl ?? resolvedPdfPage;

  // SPEC-143: shared PDF ↔ Markdown page sync controller
  const pageSync = usePageSyncController({
    initialPage: activePdfPage ?? 1,
  });

  // Inbound deeplink / chunk resolution → controller
  useEffect(() => {
    if (activePdfPage != null && activePdfPage >= 1) {
      pageSync.setPageFromExternal(activePdfPage);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- only react to URL/chunk page
  }, [activePdfPage]);

  // Debounced URL write when user navigates via the publishing pane
  useEffect(() => {
    if (pageSync.syncMode === 'none') return;
    if (pageSync.driver !== 'pdf' && pageSync.driver !== 'md') return;
    const page = pageSync.activePage;
    if (page < 1) return;
    const handle = window.setTimeout(() => {
      const params = new URLSearchParams(searchParams.toString());
      const current = params.get('page');
      if (current === String(page)) return;
      params.set('page', String(page));
      const newSearch = params.toString();
      router.replace(
        `/documents/${documentId}${newSearch ? `?${newSearch}` : ''}`,
        { scroll: false },
      );
    }, 200);
    return () => window.clearTimeout(handle);
  }, [pageSync.activePage, pageSync.driver, searchParams, router, documentId]);

  // OODA-chunk-select: Local chunk selection state for sidebar → content highlighting.
  // State is always kept in sync with the URL (`?chunk=<id>`) so any selection
  // is addressable, shareable, and survives page refresh.
  const [selectedChunkId, setSelectedChunkId] = useState<string | undefined>(chunkIdFromUrl);
  const [chunkStartLine, setChunkStartLine] = useState<number | undefined>();
  const [chunkEndLine, setChunkEndLine] = useState<number | undefined>();

  // Sync selectedChunkId when the URL param changes (e.g. user navigates to a
  // different citation deep-link without a full page reload).
  useEffect(() => {
    setSelectedChunkId(chunkIdFromUrl);
  }, [chunkIdFromUrl]);

  /**
   * Called when user clicks a chunk in the Data Hierarchy tree.
   * - Toggles chunk selection (same chunk again = deselect).
   * - Updates the URL via router.replace so the selection is deep-linkable and
   *   survives refresh / copy-paste sharing.
   * - Updates local line-range state so ContentRenderer highlights the range.
   */
  const handleChunkSelect = useCallback(
    (chunkId: string, start?: number, end?: number, page?: number) => {
      const isDeselecting = selectedChunkId === chunkId;
      const nextChunkId = isDeselecting ? undefined : chunkId;

      setSelectedChunkId(nextChunkId);
      setChunkStartLine(isDeselecting ? undefined : start);
      setChunkEndLine(isDeselecting ? undefined : end);

      // Persist selection in URL so the view is shareable / bookmarkable.
      // Use router.replace (not push) to avoid polluting the browser history
      // on every chunk click.
      const params = new URLSearchParams(searchParams.toString());
      if (nextChunkId) {
        params.set('chunk', nextChunkId);
      } else {
        params.delete('chunk');
      }
      // SPEC-033: include page in URL when chunk has page attribution.
      // This drives the PDFViewer via currentPage prop (controlled navigation).
      if (page !== undefined && page >= 1 && !isDeselecting) {
        params.set('page', String(page));
      }
      const newSearch = params.toString();
      router.replace(
        `/documents/${documentId}${newSearch ? `?${newSearch}` : ''}`,
        { scroll: false },
      );
    },
    [selectedChunkId, searchParams, router, documentId],
  );

  /**
   * Called by DocumentHierarchyTree when chunk data loads and the pre-selected
   * chunk's line range is resolved from KV lineage. Sets the active line range
   * so ContentRenderer scrolls to and highlights the chunk.
   * SRP: This does NOT toggle selection — it is a pure data resolution callback.
   */
  const handleChunkResolved = useCallback(
    (chunkId: string, start?: number, end?: number, page?: number) => {
      // Only apply if this chunk is still the active selection
      if (chunkId !== selectedChunkId) return;
      setChunkStartLine(start);
      setChunkEndLine(end);
      if (page !== undefined && page >= 1 && pageFromUrl === undefined) {
        setResolvedPdfPage(page);
      }
    },
    [selectedChunkId, pageFromUrl],
  );

  // Active line range: chunk selection overrides URL params.
  // WHY: Sidebar interaction should take precedence over deep-link defaults.
  const activeStartLine = chunkStartLine ?? startLine;
  const activeEndLine = chunkEndLine ?? endLine;

  // RP-02: Collapsible metadata sidebar state with localStorage persistence.
  // WHY: Matches the Document Preview panel collapse UX from the documents list page.
  const [isSidebarOpen, setIsSidebarOpen] = useState(() => {
    if (typeof window === 'undefined') return true;
    try {
      const stored = localStorage.getItem('document-detail-sidebar-open');
      return stored === null ? true : stored !== 'false';
    } catch {
      return true;
    }
  });
  const inFlightCollapseApplied = useRef(false);

  const toggleSidebar = useCallback(() => {
    setIsSidebarOpen((prev) => {
      const next = !prev;
      try { localStorage.setItem('document-detail-sidebar-open', String(next)); } catch { /* ignore */ }
      return next;
    });
  }, []);

  // Fetch document details.
  // SPEC-051 GAP-051-03: Use short staleTime when document is actively processing
  // so the UI updates quickly after a reprocess is triggered.
  // WHY: 30s staleTime means users see no updates for 30s after a reprocess — too long.
  const { data: document, isLoading, isError, refetch } = useQuery({
    queryKey: ['document', documentId, selectedWorkspaceId],
    queryFn: () => getDocument(documentId),
    enabled: !!documentId && !!selectedWorkspaceId,
    // Fail-fast under ingest load: surface Retry UI instead of infinite skeletons.
    retry: 1,
    // Active processing: poll every 3s. Terminal states: 30s.
    staleTime: reprocessTrackId ? 3 * 1000 : 30 * 1000,
    refetchInterval: reprocessTrackId ? 3 * 1000 : false,
    refetchOnMount: 'always',
    // SPEC-100: soft refresh keeps prior document (no full-page skeleton flash)
    placeholderData: (previous) => previous,
  });
  const coldLoad = isLoading && !document;

  // First principles: if a PDF has figure captions but no page assets yet, include
  // extracted page PNGs from the stored PDF and enrich markdown (no VLM re-OCR).
  const assetsIncludeAttempted = useRef<string | null>(null);
  useEffect(() => {
    if (!document?.id || document.source_type !== 'pdf') return;
    if (assetsIncludeAttempted.current === document.id) return;
    const md = document.content || '';
    const needsAssets =
      /figure\s+\d/i.test(md) &&
      !md.includes('assets/') &&
      !md.includes('/documents/') &&
      Boolean(document.pdf_id);
    if (!needsAssets) return;
    assetsIncludeAttempted.current = document.id;
    let cancelled = false;
    (async () => {
      try {
        const result = await includeDocumentAssetsFromPdf(document.id);
        if (!cancelled && (result.markdown_updated || result.assets_persisted > 0)) {
          await refetch();
        }
      } catch {
        // Soft-fail: viewer still shows caption text; user can refresh after backend upgrade.
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [document, refetch]);

  // OODA-91: Derive PDF ID for content fetching
  // WHY: pdf_id may be in document.pdf_id or derived from source_type
  const pdfIdForContent = resolvePdfId(document);

  // OODA-91: Fetch PDF content (markdown) separately for PDF documents
  // WHY: PDF markdown content is stored in pdf_documents table, not in regular document content
  const {
    data: pdfContent,
    isLoading: isPdfContentLoading,
    isError: isPdfContentError,
    refetch: refetchPdfContent,
  } = useQuery({
    queryKey: ['pdfContent', pdfIdForContent, selectedWorkspaceId],
    queryFn: () => getPdfContent(pdfIdForContent!),
    enabled: !!pdfIdForContent && !!selectedWorkspaceId,
    staleTime: 60 * 1000,
  });

  const handleViewInGraph = useCallback(() => {
    if (document) {
      router.push(`/graph?document=${encodeURIComponent(document.id)}`);
    }
  }, [document, router]);

  // SPEC-159: Ask about this page/document → Query with companion + seed.
  const { ask: askQuery } = useQueryHandoff();
  const handleAskAboutPage = useCallback(
    (pageOverride?: number) => {
      if (!document) return;
      const fromOverride =
        typeof pageOverride === 'number' && pageOverride >= 1
          ? pageOverride
          : undefined;
      const looksPdf =
        document.source_type === 'pdf' ||
        document.mime_type === 'application/pdf' ||
        /\.pdf$/i.test(document.file_name ?? document.title ?? '');
      const syncedPage =
        looksPdf &&
        typeof pageSync.activePage === 'number' &&
        pageSync.activePage >= 1
          ? pageSync.activePage
          : undefined;
      const page = fromOverride ?? syncedPage;
      const title =
        document.title ||
        document.file_name ||
        `Document ${document.id.slice(0, 8)}`;
      const markdown =
        pdfContent?.markdown_content || document.content || '';
      const passage =
        page !== undefined
          ? extractPageMarkdown(markdown, page) || undefined
          : undefined;
      askQuery({
        kind: 'document',
        documentId: document.id,
        title,
        page,
        passage,
      });
    },
    [askQuery, document, pageSync.activePage, pdfContent?.markdown_content],
  );

  // OODA-48: Derive PDF ID for viewer - use pdf_id if available, otherwise use document.id for PDF source types
  // WHY: The pdf_id may not be set in older documents or when source_type is 'pdf' but pdf_id wasn't populated
  const pdfIdForViewer = pdfIdForContent;
  
  // OODA-43: Detect if document is a PDF for side-by-side viewer
  // OODA-48: Require pdfIdForViewer to be truthy to prevent 'undefined' in URL
  const isPdfDocument = Boolean(pdfIdForViewer);

  // OODA-91: Create document with PDF markdown content merged in
  // WHY: PDF markdown is stored separately in pdf_documents table, not in regular document content.
  // We merge it here so ContentRenderer can display it without special PDF handling.
  // NOTE: Must be called before early returns to satisfy React Rules of Hooks
  const documentWithContent = useMemo(
    () => withPdfMarkdown(document, pdfIdForViewer, pdfContent?.markdown_content),
    [document, pdfIdForViewer, pdfContent?.markdown_content],
  );

  // SPEC-180: image documents are shown as a picture, not as their filename.
  const isImage = isImageDocument(documentWithContent ?? {});

  const pdfMarkdownMissing =
    isPdfDocument &&
    !isPdfContentLoading &&
    !documentWithContent?.content?.trim();

  const syncAvailable = hasPageMarkers(documentWithContent?.content);
  const effectiveSyncMode = syncAvailable ? pageSync.syncMode : 'none';
  const pdfCurrentPage = pdfCurrentPageForMode(
    effectiveSyncMode,
    pageSync.activePage,
  );
  const followMarkdown = syncAvailable && pageSync.followMarkdown;

  const pageHealthQuery = usePageHealth(
    isPdfDocument ? documentId : undefined,
    Boolean(isPdfDocument && document),
    { forcePollMs: reprocessTrackId ? 1500 : false },
  );

  // Clear optimistic pending once health reports running/terminal for those pages.
  useEffect(() => {
    if (pendingReprocessPages.length === 0) return;
    const pages = pageHealthQuery.data?.pages ?? [];
    if (pages.length === 0) return;
    const pending = new Set(pendingReprocessPages);
    const anyRunning = pages.some(
      (p) =>
        pending.has(p.page_number) &&
        (p.parse?.status === 'running' ||
          p.figures?.status === 'running' ||
          p.entities?.status === 'running'),
    );
    const allSettled = pendingReprocessPages.every((n) => {
      const p = pages.find((x) => x.page_number === n);
      if (!p) return false;
      const statuses = [p.parse?.status, p.figures?.status, p.entities?.status];
      return statuses.every((s) => s === 'ok' || s === 'failed' || s === 'skipped');
    });
    if (anyRunning || (allSettled && !reprocessTrackId)) {
      setPendingReprocessPages([]);
    }
  }, [
    pageHealthQuery.data?.pages,
    pendingReprocessPages,
    reprocessTrackId,
  ]);

  const lifecycle = useMemo(
    () =>
      resolveDetailLifecycle({
        status: document?.status,
        display_status: document?.display_status,
        current_stage: document?.current_stage,
        ui_phase: document?.ui_phase,
        track_id: document?.track_id,
        stage_message: document?.stage_message,
        progress_counts: document?.progress_counts as
          | { unit?: string; current?: number; total?: number }
          | null
          | undefined,
        page_count: document?.page_count,
      }),
    [document],
  );

  // Viewer-first: collapse Details once when we first see an in-flight doc.
  useEffect(() => {
    if (!document || inFlightCollapseApplied.current) return;
    if (lifecycle.isInFlight) {
      setIsSidebarOpen(false);
      inFlightCollapseApplied.current = true;
    }
  }, [document, lifecycle.isInFlight]);

  const isFailed = lifecycle.kind === 'failed' || lifecycle.kind === 'partial';
  const isCancelled = lifecycle.kind === 'cancelled';

  // Loading state — SPEC-100: match final 2-column shell (CLS)
  if (coldLoad) {
    return (
      <div
        className="flex h-full min-h-0 flex-col overflow-clip"
        data-testid="spec100-document-detail-skeleton"
      >
        <HeaderSkeleton />
        {/* Idle progress slot collapsed — matches live page (no dead band before body). */}
        <div
          className="h-0 min-h-0 overflow-hidden"
          data-testid="detail-page-reprocess-progress-slot"
          aria-hidden
        />
        <div className="flex min-h-0 flex-1">
          <div className="flex min-h-0 flex-1 flex-col p-page">
            <Skeleton className="mb-3 h-8 w-1/2" />
            <Skeleton className="min-h-0 flex-1 w-full" />
          </div>
          <div className="hidden w-[35%] shrink-0 border-l px-page py-page md:block">
            <Skeleton className="mb-4 h-32 w-full" />
            <Skeleton className="h-48 w-full" />
            <Skeleton className="mt-4 h-24 w-full" />
          </div>
        </div>
      </div>
    );
  }

  // Error state
  if (isError || !document || !documentWithContent) {
    return (
      <div className="flex flex-col h-full">
        <ErrorHeader />
        <div className="flex-1 flex items-center justify-center p-8">
          <ErrorContent onRetry={refetch} />
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Minimal Header */}
      <header className="shrink-0 border-b bg-background">
        <div className="flex items-center justify-between px-3 py-2">
          <div className="flex items-center gap-2 min-w-0 flex-1">
            <Button variant="ghost" size="icon" className="h-8 w-8" asChild>
              <Link href="/documents">
                <ArrowLeft className="h-4 w-4" />
              </Link>
            </Button>
            
            <div className="min-w-0 flex-1">
              <h1 className="text-base font-semibold truncate">
                {document.title || document.file_name || `Document ${document.id.slice(0, 8)}`}
              </h1>
            </div>
          </div>
          
          <div className="flex items-center gap-1 shrink-0">
            <DetailLifecycleBadge lifecycle={lifecycle} />
            <DocumentDownloadMenu
              document={documentWithContent}
              markdownContent={documentWithContent.content}
              variant="icon"
            />
            <Button
              variant="ghost"
              size="sm"
              className="h-8 gap-1.5 px-2"
              onClick={() => handleAskAboutPage()}
              title={
                typeof pageSync.activePage === 'number' && pageSync.activePage >= 1
                  ? t('documents.detail.askAboutPage', 'Ask about this page')
                  : t('documents.detail.askAboutDocument', 'Ask about this document')
              }
              aria-label={
                typeof pageSync.activePage === 'number' && pageSync.activePage >= 1
                  ? t('documents.detail.askAboutPage', 'Ask about this page')
                  : t('documents.detail.askAboutDocument', 'Ask about this document')
              }
              data-testid="detail-ask-about-page"
            >
              <MessageSquareText className="h-3.5 w-3.5" />
              <span className="hidden sm:inline text-xs">
                {t('documents.detail.askShort', 'Ask')}
              </span>
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={handleViewInGraph}
              title={t('documents.detail.viewInGraph', 'View in knowledge graph')}
              aria-label={t('documents.detail.viewInGraph', 'View in knowledge graph')}
              data-testid="detail-view-in-graph"
            >
              <Network className="h-3.5 w-3.5" />
            </Button>
            {/* SPEC-051 GAP-051-01: Reprocess — hidden while in-flight (lifecycle SSOT). */}
            {isPdfDocument && lifecycle.canReprocessPages && (
              <Button
                variant="default"
                size="sm"
                className="h-8 gap-1.5"
                disabled={Boolean(reprocessTrackId)}
                onClick={() => {
                  setSelectedPages([]);
                  setPagesReprocessOpen(true);
                }}
                data-testid="detail-page-reprocess-pages-button"
              >
                <RefreshCw
                  className={`h-3.5 w-3.5${reprocessTrackId ? ' animate-spin' : ''}`}
                />
                {reprocessTrackId
                  ? t('documents.pageHealth.reprocessingBusy', 'Reprocessing…')
                  : t(
                      'documents.pageHealth.menuAction',
                      'Reprocess specific pages',
                    )}
              </Button>
            )}
            {lifecycle.canReprocess && !reprocessMutationDetail.isPending && (
              <Button
                variant="outline"
                size="sm"
                className="h-8 gap-1.5"
                onClick={() => setReprocessDialogOpen(true)}
                data-testid="detail-page-reprocess-button"
              >
                <RotateCcw className="h-3.5 w-3.5" />
                {t('documents.reprocess.action', 'Reprocess')}
              </Button>
            )}
            {lifecycle.canCancel && document && (
              <Button
                variant="outline"
                size="sm"
                className="h-8 gap-1.5 text-destructive border-destructive/30 hover:bg-destructive/10"
                onClick={() => cancelMutationDetail.mutate(document)}
                disabled={cancelMutationDetail.isPending}
                data-testid="detail-page-cancel-button"
              >
                {cancelMutationDetail.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <StopCircle className="h-3.5 w-3.5" />
                )}
                {t('documents.cancel.action', 'Cancel')}
              </Button>
            )}
          </div>
        </div>

        {isFailed && getEffectiveErrorMessage(document) && (
          <div className="px-3 py-2 bg-destructive/10 border-t">
            <p className="text-xs text-destructive break-words overflow-hidden">
              {getEffectiveErrorMessage(document)}
            </p>
          </div>
        )}
        {isCancelled && (
          <div className="px-3 py-2 bg-muted/50 border-t">
            <p className="text-xs text-muted-foreground">
              {t('documents.cancelled.message', 'Processing was cancelled. Click Reprocess to retry.')}
            </p>
          </div>
        )}
        {/* Progress slot: always mounted. Idle height 0 — an empty h-5.5rem
            reserve was a dead white band before the document body ("gap before
            the text"). Expanding on reprocess is user-initiated (hadRecentInput). */}
        <div
          className={
            reprocessTrackId
              ? 'min-h-[5.5rem] shrink-0 border-t bg-card/80 px-3 py-2'
              : 'h-0 min-h-0 overflow-hidden border-0 p-0'
          }
          data-testid="detail-page-reprocess-progress-slot"
          aria-hidden={!reprocessTrackId}
        >
          {reprocessTrackId ? (
            <div data-testid="detail-page-reprocess-progress">
              <ProgressPanelRow
                trackId={resolveReprocessPanelTrackId(
                  reprocessTrackId,
                  document?.track_id,
                )}
                documentName={
                  document?.file_name || document?.title || documentId.slice(0, 8)
                }
                isPdf={shouldUsePdfReprocessPanel(
                  document?.source_type === 'pdf',
                  reprocessMode,
                )}
                onComplete={() => {
                  setReprocessTrackId(null);
                  setPendingReprocessPages([]);
                  void refetch();
                  void pageHealthQuery.refetch();
                }}
                onFailed={() => {
                  setReprocessTrackId(null);
                  setPendingReprocessPages([]);
                  void pageHealthQuery.refetch();
                }}
                onCancel={() => {
                  setReprocessTrackId(null);
                  setPendingReprocessPages([]);
                }}
                data-testid="detail-page-reprocess-panel"
              />
            </div>
          ) : null}
        </div>
      </header>

      {/* SPEC-051: Reprocess choice dialog for the document detail page. */}
      <ReprocessDialog
        open={reprocessDialogOpen}
        document={document ? {
          id: document.id,
          title: document.title,
          file_name: document.file_name,
          source_type: document.source_type as 'pdf' | 'text',
          status: document.status,
          document_type: document.document_type ?? undefined,
          mime_type: document.mime_type ?? undefined,
        } : null}
        onConfirm={(choice: ReprocessChoice) => {
          setReprocessDialogOpen(false);
          if (choice.mode === 'pages') {
            setSelectedPages([]);
            setPagesReprocessOpen(true);
            return;
          }
          reprocessMutationDetail.mutate({ mode: choice.mode });
        }}
        onCancel={() => setReprocessDialogOpen(false)}
      />

      <ReprocessPagesDialog
        open={pagesReprocessOpen}
        documentId={documentId}
        documentName={
          document?.file_name || document?.title || undefined
        }
        pageCount={document?.page_count ?? pageHealthQuery.data?.page_count}
        initialPages={selectedPages}
        onClose={() => {
          setPagesReprocessOpen(false);
          setSelectedPages([]);
        }}
        onQueued={(trackId, pages) => {
          setPendingReprocessPages(pages);
          setSelectedPages([]);
          setReprocessTrackId(trackId);
          setReprocessMode('entities');
          void pageHealthQuery.refetch();
        }}
      />

      {/* Main Content Area - Two Column Layout
          WHY (layout): Viewer + Details must be flex-row siblings. Nesting Details
          inside the viewer flex-col stacked it under the PDF and let lineage height
          collapse the viewer to empty whitespace (broken detail view). */}
      <div className="flex min-h-0 flex-1 overflow-hidden">
        {isLgUp ? (
        <div className="flex min-h-0 flex-1 flex-row overflow-hidden">
          {/* Main column: page health + document viewer */}
          <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
            {/* Quiet header: no idle page selector. Progress banner only while a
                page-reprocess track is live (CTA lives in the document header). */}
            {isPdfDocument &&
            Boolean(reprocessTrackId) &&
            (pageHealthQuery.data?.pages?.length ?? 0) > 0 ? (
              <div className="shrink-0 border-b px-3 py-2">
                <PageHealthStrip
                  mode="overview"
                  pages={pageHealthQuery.data!.pages}
                  reprocessActive
                  pendingPages={pendingReprocessPages}
                  hideChrome
                  hideMeters
                  hideTiles
                />
              </div>
            ) : null}
            {/* Content Area fills remaining height beside Details */}
            <div
              className={
                isPdfDocument
                  ? 'min-h-0 flex-1 overflow-hidden'
                  : 'min-h-0 flex-1 overflow-auto'
              }
            >
              {isPdfDocument ? (
                /* OODA-43: PDF documents show side-by-side PDF and Markdown viewer */
                <SideBySideViewer
                  height={undefined}
                  className="h-full"
                  leftTitle="PDF Document"
                  rightTitle="Extracted Markdown"
                  syncMode={pageSync.syncMode}
                  onSyncModeChange={pageSync.setSyncMode}
                  syncAvailable={syncAvailable}
                  leftPanel={
                    // OODA-48: Use pdfIdForViewer which is guaranteed to exist when isPdfDocument is true
                    <PDFViewer
                      file={getPdfDownloadUrl(pdfIdForViewer!)}
                      initialPage={pageSync.activePage}
                      currentPage={pdfCurrentPage}
                      onPageChange={pageSync.setPageFromPdf}
                      onGestureStart={() => pageSync.beginGesture('pdf')}
                      onGestureEnd={pageSync.endGesture}
                      documentId={documentId}
                      onAskAboutPage={handleAskAboutPage}
                    />
                  }
                  rightPanel={
                    // OODA-91: Show loading state while PDF markdown is being fetched
                    isPdfContentLoading ? (
                      <div className="flex h-full items-center justify-center">
                        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                      </div>
                    ) : pdfMarkdownMissing ? (
                      <PdfMarkdownEmptyState
                        mode={lifecycle.emptyMarkdownMode}
                        isError={isPdfContentError}
                        onRetry={() => {
                          void refetchPdfContent();
                          void refetch();
                        }}
                        onReprocessPages={
                          lifecycle.canReprocessPages
                            ? () => setPagesReprocessOpen(true)
                            : undefined
                        }
                      />
                    ) : (
                      <ContentRenderer
                        document={documentWithContent}
                        highlightText={highlightText}
                        startLine={activeStartLine}
                        endLine={activeEndLine}
                        activePage={pageSync.activePage}
                        syncEnabled={pageSync.syncEnabled && syncAvailable}
                        followMarkdown={followMarkdown}
                        onPageFromMd={pageSync.setPageFromMd}
                        syncDriver={pageSync.driver}
                        onMdGestureStart={() => pageSync.beginGesture('md')}
                        onMdGestureEnd={pageSync.endGesture}
                      />
                    )
                  }
                />
              ) : isImage ? (
                /* SPEC-180: an image document's `content` is its file name; the pixels are
                   the payload. Render the picture, never the filename as body text. */
                <DocumentImageViewer document={document} className="h-full" />
              ) : (
                /* Non-PDF documents show ContentRenderer only */
                <ContentRenderer
                  document={documentWithContent}
                  highlightText={highlightText}
                  startLine={activeStartLine}
                  endLine={activeEndLine}
                />
              )}
            </div>
          </div>

          {/* Metadata Sidebar - sibling of main column (RP-02 / SPEC-040). */}
          {!isSidebarOpen ? (
            <div
              className="flex w-10 shrink-0 cursor-pointer flex-col items-center border-l bg-card/50 py-4 transition-colors hover:bg-muted/70"
              onClick={toggleSidebar}
              role="button"
              tabIndex={0}
              aria-label="Expand details panel"
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  toggleSidebar();
                }
              }}
            >
              <ChevronLeft className="mb-2 h-4 w-4 text-muted-foreground" />
              <span
                className="text-xs text-muted-foreground"
                style={{ writingMode: 'vertical-rl', transform: 'rotate(180deg)' }}
              >
                Details
              </span>
            </div>
          ) : (
            <ResizablePanel
              side="right"
              defaultWidth={360}
              minWidth={280}
              maxWidth={560}
              storageKey="document-detail-sidebar-width"
              ariaLabel="Resize metadata sidebar"
              className="h-full min-h-0"
            >
              <div className="flex h-full min-h-0 flex-col overflow-hidden border-l bg-background">
                <div className="flex shrink-0 items-center justify-between border-b bg-muted/20 px-3 py-1.5">
                  <span className="text-xs font-medium text-muted-foreground">
                    Details
                  </span>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6"
                    onClick={toggleSidebar}
                    aria-label="Collapse details panel"
                  >
                    <ChevronRight className="h-3.5 w-3.5" />
                  </Button>
                </div>
                <div className="min-h-0 flex-1 overflow-hidden">
                  <MetadataSidebar
                    document={document}
                    onChunkSelect={handleChunkSelect}
                    onChunkResolved={handleChunkResolved}
                    selectedChunkId={selectedChunkId}
                    className="border-l-0"
                  />
                </div>
              </div>
            </ResizablePanel>
          )}
        </div>
        ) : (
        <div className="flex-1 overflow-hidden">
          <Tabs defaultValue="content" className="h-full flex flex-col">
            {/* SPEC-100: always 3-col tab slot so async PDF detection does not widen tabs. */}
            <TabsList
              className="grid w-full grid-cols-3 rounded-none border-b"
              data-testid="spec100-document-detail-tabs"
            >
              <TabsTrigger
                value="pdf"
                disabled={!isPdfDocument}
                className={!isPdfDocument ? 'invisible pointer-events-none' : undefined}
              >
                PDF
              </TabsTrigger>
              <TabsTrigger value="content">Markdown</TabsTrigger>
              <TabsTrigger value="metadata">Details</TabsTrigger>
            </TabsList>
            {isPdfDocument ? (
              <div className="flex shrink-0 items-center justify-end border-b px-2 py-1">
                <PageSyncModeControl
                  mode={pageSync.syncMode}
                  onModeChange={pageSync.setSyncMode}
                  available={syncAvailable}
                  compact
                  testId="pdf-md-sync-mode-mobile"
                />
              </div>
            ) : null}
            {/* OODA-48: Use pdfIdForViewer which is guaranteed to exist when isPdfDocument is true */}
            {isPdfDocument && pdfIdForViewer && (
              <TabsContent value="pdf" className="flex-1 overflow-hidden m-0 mt-0">
                <PDFViewer
                  file={getPdfDownloadUrl(pdfIdForViewer)}
                  initialPage={pageSync.activePage}
                  currentPage={pdfCurrentPage}
                  onPageChange={pageSync.setPageFromPdf}
                  onGestureStart={() => pageSync.beginGesture('pdf')}
                  onGestureEnd={pageSync.endGesture}
                  documentId={documentId}
                  onAskAboutPage={handleAskAboutPage}
                />
              </TabsContent>
            )}
            <TabsContent
              value="content"
              className="flex-1 overflow-auto m-0 mt-0"
              data-testid="md-scroll-container-mobile"
            >
              {/* OODA-91: Show loading state for PDF markdown on mobile */}
              {isPdfDocument && isPdfContentLoading ? (
                <div className="flex items-center justify-center h-full">
                  <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                </div>
              ) : pdfMarkdownMissing ? (
                <PdfMarkdownEmptyState
                  mode={lifecycle.emptyMarkdownMode}
                  isError={isPdfContentError}
                  onRetry={() => {
                    void refetchPdfContent();
                    void refetch();
                  }}
                  onReprocessPages={
                    lifecycle.canReprocessPages
                      ? () => setPagesReprocessOpen(true)
                      : undefined
                  }
                />
              ) : isImage ? (
                <DocumentImageViewer document={document} className="h-full" />
              ) : (
                <ContentRenderer
                  document={documentWithContent}
                  highlightText={highlightText}
                  startLine={activeStartLine}
                  endLine={activeEndLine}
                  activePage={pageSync.activePage}
                  syncEnabled={pageSync.syncEnabled && syncAvailable}
                  followMarkdown={followMarkdown}
                  onPageFromMd={pageSync.setPageFromMd}
                  syncDriver={pageSync.driver}
                  onMdGestureStart={() => pageSync.beginGesture('md')}
                  onMdGestureEnd={pageSync.endGesture}
                />
              )}
            </TabsContent>
            <TabsContent value="metadata" className="flex-1 overflow-hidden m-0 mt-0">
              <MetadataSidebar
                document={document}
                onChunkSelect={handleChunkSelect}
                onChunkResolved={handleChunkResolved}
                selectedChunkId={selectedChunkId}
              />
            </TabsContent>
          </Tabs>
        </div>
        )}
      </div>
    </div>
  );
}

function PdfMarkdownEmptyState({
  mode,
  isError,
  onRetry,
  onReprocessPages,
}: {
  mode: 'in_flight' | 'failed' | 'missing';
  isError: boolean;
  onRetry: () => void;
  onReprocessPages?: () => void;
}) {
  const { t } = useTranslation();
  const inFlight = mode === 'in_flight' && !isError;
  const title = isError
    ? t('documents.pdf.markdownUnavailable', 'Extracted markdown is not available yet')
    : inFlight
      ? t(
          'documents.pdf.emptyMarkdown.inFlightTitle',
          'Markdown is preparing…',
        )
      : mode === 'failed'
        ? t(
            'documents.pdf.emptyMarkdown.failedTitle',
            'No markdown to show',
          )
        : t(
            'documents.pdf.emptyMarkdown.missingTitle',
            'No markdown stored',
          );
  const body = isError
    ? t(
        'documents.pdf.markdownLoadError',
        'Could not load markdown from the server. Retry or reprocess the document.',
      )
    : inFlight
      ? t(
          'documents.pdf.emptyMarkdown.inFlight',
          'Convert is running. Text appears here as pages finish.',
        )
      : mode === 'failed'
        ? t(
            'documents.pdf.emptyMarkdown.failed',
            'Processing stopped before markdown was stored. Reprocess pages or the full document to recover.',
          )
        : t(
            'documents.pdf.emptyMarkdown.missing',
            'This PDF has no stored markdown. Reprocess to generate it.',
          );

  return (
    <div
      className="flex flex-col items-center justify-center h-full gap-4 p-8 text-center"
      data-testid="pdf-markdown-empty-state"
      data-mode={isError ? 'error' : mode}
    >
      {inFlight ? (
        <Loader2 className="h-10 w-10 animate-spin text-muted-foreground" />
      ) : mode === 'failed' || isError ? (
        <AlertCircle className="h-10 w-10 text-muted-foreground" />
      ) : (
        <FileText className="h-10 w-10 text-muted-foreground" />
      )}
      <div className="space-y-2 max-w-md">
        <p className="text-sm font-medium">{title}</p>
        <p className="text-xs text-muted-foreground">{body}</p>
      </div>
      {!inFlight ? (
        <div className="flex flex-wrap items-center justify-center gap-2">
          <Button variant="outline" size="sm" onClick={onRetry}>
            <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
            {t('common.retry', 'Retry')}
          </Button>
          {onReprocessPages ? (
            <Button variant="default" size="sm" onClick={onReprocessPages}>
              <RotateCcw className="h-3.5 w-3.5 mr-1.5" />
              {t('documents.pageHealth.menuAction', 'Reprocess specific pages')}
            </Button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}

function HeaderSkeleton() {
  return (
    <div className="border-b bg-background p-4">
      <div className="flex items-center gap-3">
        <Skeleton className="h-9 w-9" />
        <Skeleton className="h-6 w-64" />
      </div>
    </div>
  );
}

function ErrorHeader() {
  return (
    <div className="border-b bg-background p-4">
      <div className="flex items-center gap-3">
        <Button variant="ghost" size="icon" asChild>
          <Link href="/documents">
            <ArrowLeft className="h-4 w-4" />
          </Link>
        </Button>
        <h1 className="text-lg font-semibold">Document</h1>
      </div>
    </div>
  );
}

function ErrorContent({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="text-center max-w-md">
      <div className="rounded-full bg-red-500/10 p-4 w-fit mx-auto mb-4">
        <AlertCircle className="h-8 w-8 text-red-500" />
      </div>
      {/* Same copy for a missing id and a hidden one (SPEC-174): never reveal which. */}
      <h2 className="text-xl font-semibold mb-2">Document not found</h2>
      <p className="text-muted-foreground mb-4">
        It may have been deleted, or the link is wrong.
      </p>
      <div className="flex gap-2 justify-center">
        <Button variant="outline" onClick={onRetry}>
          <RefreshCw className="h-4 w-4 mr-2" />
          Retry
        </Button>
        <Button asChild>
          <Link href="/documents">Back to Documents</Link>
        </Button>
      </div>
    </div>
  );
}
