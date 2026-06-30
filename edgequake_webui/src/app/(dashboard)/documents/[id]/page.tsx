'use client';

import { ContentRenderer } from '@/components/document/content-renderer';
import { MetadataSidebar } from '@/components/document/metadata-sidebar';
import { PDFViewer } from '@/components/documents/pdf-viewer';
import { SideBySideViewer } from '@/components/documents/side-by-side-viewer';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ResizablePanel } from '@/components/ui/resizable-panel';
import { Skeleton } from '@/components/ui/skeleton';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { getDocument, getPdfContent, getPdfDownloadUrl } from '@/lib/api/edgequake';
import { getEffectiveErrorMessage } from '@/lib/utils/document-status';
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import {
    AlertCircle,
    ArrowLeft,
    ChevronLeft,
    ChevronRight,
    Download,
    Loader2,
    Network,
    RefreshCw,
    StopCircle,
} from 'lucide-react';
import Link from 'next/link';
import { useParams, useRouter, useSearchParams } from 'next/navigation';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

type DocumentStatus =
  | 'pending'
  | 'processing'
  | 'completed'
  | 'indexed'
  | 'partial_failure'
  | 'failed'
  | 'cancelled';

export default function DocumentViewPage() {
  const { t } = useTranslation();
  const router = useRouter();
  const params = useParams();
  const searchParams = useSearchParams();
  const documentId = params.id as string;
  const { selectedWorkspaceId } = useTenantStore();
  
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
  const pageFromUrl = useMemo(() => {
    const paramPage = searchParams.get('page');
    if (paramPage) {
      const n = parseInt(paramPage, 10);
      if (!isNaN(n) && n >= 1) return n;
    }
    return undefined;
  }, [searchParams]);

  const [initialPdfPage, setInitialPdfPage] = useState<number>(pageFromUrl ?? 1);

  // Sync initialPdfPage when URL param changes (e.g. user clicks different citations)
  useEffect(() => {
    if (pageFromUrl !== undefined) {
      setInitialPdfPage(pageFromUrl);
    }
  }, [pageFromUrl]);
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
    (chunkId: string, start?: number, end?: number) => {
      // Only apply if this chunk is still the active selection
      if (chunkId !== selectedChunkId) return;
      setChunkStartLine(start);
      setChunkEndLine(end);
    },
    [selectedChunkId],
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

  const toggleSidebar = useCallback(() => {
    setIsSidebarOpen((prev) => {
      const next = !prev;
      try { localStorage.setItem('document-detail-sidebar-open', String(next)); } catch { /* ignore */ }
      return next;
    });
  }, []);

  // Fetch document details
  const { data: document, isLoading, isError, error, refetch } = useQuery({
    queryKey: ['document', documentId, selectedWorkspaceId],
    queryFn: () => getDocument(documentId),
    enabled: !!documentId && !!selectedWorkspaceId,
    staleTime: 30 * 1000,
    refetchOnMount: 'always',
  });

  // OODA-91: Derive PDF ID for content fetching
  // WHY: pdf_id may be in document.pdf_id or derived from source_type
  const pdfIdForContent = document?.pdf_id || (document?.source_type === 'pdf' ? document?.id : null);

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
      router.push(`/graph?highlight=${document.id}`);
    }
  }, [document, router]);

  // OODA-48: Derive PDF ID for viewer - use pdf_id if available, otherwise use document.id for PDF source types
  // WHY: The pdf_id may not be set in older documents or when source_type is 'pdf' but pdf_id wasn't populated
  const pdfIdForViewer = document?.pdf_id || (document?.source_type === 'pdf' ? document?.id : null);
  
  // OODA-43: Detect if document is a PDF for side-by-side viewer
  // OODA-48: Require pdfIdForViewer to be truthy to prevent 'undefined' in URL
  const isPdfDocument = Boolean(pdfIdForViewer);

  // OODA-91: Create document with PDF markdown content merged in
  // WHY: PDF markdown is stored separately in pdf_documents table, not in regular document content.
  // We merge it here so ContentRenderer can display it without special PDF handling.
  // NOTE: Must be called before early returns to satisfy React Rules of Hooks
  const documentWithContent = useMemo(() => {
    if (!document) return null;
    const markdown =
      (pdfContent?.markdown_content?.trim() || document.content?.trim() || '') as string;
    if (isPdfDocument && markdown) {
      return {
        ...document,
        content: markdown,
        // WHY: PDF mime routes to plain-text path; markdown path needs text/markdown or signatures.
        mime_type: 'text/markdown',
        source_type: 'pdf' as const,
      };
    }
    return document;
  }, [document, isPdfDocument, pdfContent?.markdown_content]);

  const pdfMarkdownMissing =
    isPdfDocument &&
    !isPdfContentLoading &&
    !documentWithContent?.content?.trim();

  // Derived status values (safe to compute even if document is null)
  const status = (document?.status || 'completed') as DocumentStatus;
  const isFailed = status === 'failed' || status === 'partial_failure';
  const isCancelled = status === 'cancelled';

  // Loading state
  if (isLoading) {
    return (
      <div className="flex flex-col h-full">
        <HeaderSkeleton />
        <div className="flex-1 flex">
          <div className="flex-1 p-8">
            <Skeleton className="h-32 w-full mb-4" />
            <Skeleton className="h-64 w-full" />
          </div>
          <div className="w-[35%] border-l p-4">
            <Skeleton className="h-32 w-full mb-4" />
            <Skeleton className="h-48 w-full" />
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
          <ErrorContent error={error as Error} onRetry={refetch} />
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
            {status === 'processing' && (
              <Badge variant="outline" className="text-xs">
                <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                Processing
              </Badge>
            )}
            {status === 'partial_failure' && (
              <Badge variant="outline" className="text-xs border-orange-500 text-orange-500">
                <AlertCircle className="h-3 w-3 mr-1" />
                Partial Failure
              </Badge>
            )}
            {status === 'failed' && (
              <Badge variant="destructive" className="text-xs">
                <AlertCircle className="h-3 w-3 mr-1" />
                Failed
              </Badge>
            )}
            {isCancelled && (
              <Badge variant="outline" className="text-xs border-gray-500 text-gray-500">
                <StopCircle className="h-3 w-3 mr-1" />
                Cancelled
              </Badge>
            )}
            {isPdfDocument && pdfIdForViewer && (
              <Button variant="ghost" size="sm" className="h-8" asChild>
                <a href={getPdfDownloadUrl(pdfIdForViewer)} target="_blank" rel="noopener noreferrer">
                  <Download className="h-3.5 w-3.5" />
                </a>
              </Button>
            )}
            <Button variant="ghost" size="sm" className="h-8" onClick={handleViewInGraph}>
              <Network className="h-3.5 w-3.5" />
            </Button>
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
              {t('documents.cancelled.message', 'Processing was cancelled. You can reprocess this document from the documents list.')}
            </p>
          </div>
        )}
      </header>

      {/* Main Content Area - Two Column Layout */}
      <div className="flex-1 flex overflow-hidden">
        {/* OODA-43: Desktop layout with PDF side-by-side support */}
        <div className="hidden lg:flex flex-1 overflow-hidden">
          {/* Content Area - 65% (or full width for PDF side-by-side) */}
          <div className={isPdfDocument ? "flex-1 overflow-hidden" : "flex-1 overflow-auto"}>
            {isPdfDocument ? (
              /* OODA-43: PDF documents show side-by-side PDF and Markdown viewer */
              <SideBySideViewer
                height={undefined}
                className="h-full"
                leftTitle="PDF Document"
                rightTitle="Extracted Markdown"
                leftPanel={
                  // OODA-48: Use pdfIdForViewer which is guaranteed to exist when isPdfDocument is true
                  <PDFViewer
                    file={getPdfDownloadUrl(pdfIdForViewer!)}
                    initialPage={initialPdfPage}
                    currentPage={pageFromUrl}
                  />
                }
                rightPanel={
                  // OODA-91: Show loading state while PDF markdown is being fetched
                  isPdfContentLoading ? (
                    <div className="flex items-center justify-center h-full">
                      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                    </div>
                  ) : pdfMarkdownMissing ? (
                    <PdfMarkdownEmptyState
                      isError={isPdfContentError}
                      onRetry={() => {
                        void refetchPdfContent();
                        void refetch();
                      }}
                    />
                  ) : (
                    <ContentRenderer 
                      document={documentWithContent} 
                      highlightText={highlightText}
                      startLine={activeStartLine}
                      endLine={activeEndLine}
                    />
                  )
                }
              />
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

          {/* Metadata Sidebar - Resizable + Collapsible (RP-02).
              WHY: The sidebar contains the LineageTree which shows the Vision LLM
              used for PDF → Markdown transcription. Hiding it for PDF documents
              would make lineage information inaccessible to the user.
              SPEC-040: Vision LLM lineage must be visible in document detail view. */}
          {!isSidebarOpen ? (
            /* Collapsed state: thin bar matching RightPanel collapse style */
            <div
              className="w-10 border-l bg-card/50 flex flex-col items-center py-4 cursor-pointer hover:bg-muted/70 transition-colors shrink-0"
              onClick={toggleSidebar}
              role="button"
              tabIndex={0}
              aria-label="Expand details panel"
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); toggleSidebar(); }
              }}
            >
              <ChevronLeft className="h-4 w-4 text-muted-foreground mb-2" />
              <span
                className="text-xs text-muted-foreground"
                style={{ writingMode: 'vertical-rl', transform: 'rotate(180deg)' }}
              >
                Details
              </span>
            </div>
          ) : (
            /* Expanded state: resizable panel with collapse toggle */
            <ResizablePanel
              side="right"
              defaultWidth={400}
              minWidth={280}
              maxWidth={700}
              storageKey="document-detail-sidebar-width"
              ariaLabel="Resize metadata sidebar"
            >
              {/* border-l here so the full panel edge (incl. strip) has the separator */}
              <div className="flex flex-col h-full overflow-hidden border-l bg-background">
                {/* Collapse toggle strip */}
                <div className="flex items-center justify-between px-3 py-1.5 shrink-0 border-b bg-muted/20">
                  <span className="text-xs font-medium text-muted-foreground">Details</span>
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
                <div className="flex-1 overflow-hidden">
                  {/* border-l-0: outer wrapper already provides the left border */}
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

        {/* Mobile/Tablet: Tabbed layout */}
        <div className="flex-1 lg:hidden overflow-hidden">
          <Tabs defaultValue="content" className="h-full flex flex-col">
            <TabsList className={`grid w-full ${isPdfDocument ? 'grid-cols-3' : 'grid-cols-2'} rounded-none border-b`}>
              {isPdfDocument && <TabsTrigger value="pdf">PDF</TabsTrigger>}
              <TabsTrigger value="content">Markdown</TabsTrigger>
              <TabsTrigger value="metadata">Details</TabsTrigger>
            </TabsList>
            {/* OODA-48: Use pdfIdForViewer which is guaranteed to exist when isPdfDocument is true */}
            {isPdfDocument && pdfIdForViewer && (
              <TabsContent value="pdf" className="flex-1 overflow-hidden m-0 mt-0">
                <PDFViewer
                  file={getPdfDownloadUrl(pdfIdForViewer)}
                  initialPage={initialPdfPage}
                  currentPage={pageFromUrl}
                />
              </TabsContent>
            )}
            <TabsContent value="content" className="flex-1 overflow-auto m-0 mt-0">
              {/* OODA-91: Show loading state for PDF markdown on mobile */}
              {isPdfDocument && isPdfContentLoading ? (
                <div className="flex items-center justify-center h-full">
                  <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                </div>
              ) : pdfMarkdownMissing ? (
                <PdfMarkdownEmptyState
                  isError={isPdfContentError}
                  onRetry={() => {
                    void refetchPdfContent();
                    void refetch();
                  }}
                />
              ) : (
                <ContentRenderer 
                  document={documentWithContent} 
                  highlightText={highlightText}
                  startLine={activeStartLine}
                  endLine={activeEndLine}
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
      </div>
    </div>
  );
}

function PdfMarkdownEmptyState({
  isError,
  onRetry,
}: {
  isError: boolean;
  onRetry: () => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-center justify-center h-full gap-4 p-8 text-center">
      <AlertCircle className="h-10 w-10 text-muted-foreground" />
      <div className="space-y-2 max-w-md">
        <p className="text-sm font-medium">
          {t(
            'documents.pdf.markdownUnavailable',
            'Extracted markdown is not available yet',
          )}
        </p>
        <p className="text-xs text-muted-foreground">
          {isError
            ? t(
                'documents.pdf.markdownLoadError',
                'Could not load markdown from the server. Retry or reprocess the document.',
              )
            : t(
                'documents.pdf.markdownPending',
                'Processing may still be running, or markdown was not stored. Try refresh or reprocess from the documents list.',
              )}
        </p>
      </div>
      <Button variant="outline" size="sm" onClick={onRetry}>
        <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
        {t('common.retry', 'Retry')}
      </Button>
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
        <h1 className="text-lg font-semibold">Document Not Found</h1>
      </div>
    </div>
  );
}

function ErrorContent({ error, onRetry }: { error: Error; onRetry: () => void }) {
  return (
    <div className="text-center max-w-md">
      <div className="rounded-full bg-red-500/10 p-4 w-fit mx-auto mb-4">
        <AlertCircle className="h-8 w-8 text-red-500" />
      </div>
      <h2 className="text-xl font-semibold mb-2">Document Not Found</h2>
      <p className="text-muted-foreground mb-4">
        {error?.message || 'The document you are looking for could not be found or you may not have access to it.'}
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
