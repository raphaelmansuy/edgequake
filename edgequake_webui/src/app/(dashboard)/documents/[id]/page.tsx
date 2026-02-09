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
import { useTenantStore } from '@/stores/use-tenant-store';
import { useQuery } from '@tanstack/react-query';
import {
    AlertCircle,
    ArrowLeft,
    CheckCircle,
    Clock,
    Download,
    Loader2,
    Network,
    RefreshCw,
    XCircle
} from 'lucide-react';
import Link from 'next/link';
import { useParams, useRouter, useSearchParams } from 'next/navigation';
import { useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';

const statusConfig = {
  pending: { icon: Clock, color: 'text-yellow-500', bg: 'bg-yellow-500/10', label: 'Pending' },
  processing: { icon: Loader2, color: 'text-blue-500', bg: 'bg-blue-500/10', label: 'Processing' },
  completed: { icon: CheckCircle, color: 'text-green-500', bg: 'bg-green-500/10', label: 'Completed' },
  indexed: { icon: CheckCircle, color: 'text-green-500', bg: 'bg-green-500/10', label: 'Indexed' },
  partial_failure: { icon: AlertCircle, color: 'text-orange-500', bg: 'bg-orange-500/10', label: 'Partial Failure' },
  failed: { icon: XCircle, color: 'text-red-500', bg: 'bg-red-500/10', label: 'Failed' },
  cancelled: { icon: XCircle, color: 'text-gray-500', bg: 'bg-gray-500/10', label: 'Cancelled' },
} as const;

type DocumentStatus = keyof typeof statusConfig;

function formatFileSize(bytes: number | undefined): string {
  if (!bytes) return 'Unknown';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
}

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
  const { data: pdfContent, isLoading: isPdfContentLoading } = useQuery({
    queryKey: ['pdfContent', pdfIdForContent],
    queryFn: () => getPdfContent(pdfIdForContent!),
    enabled: !!pdfIdForContent,
    staleTime: 60 * 1000,
  });

  const handleCopyId = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(documentId);
      toast.success(t('documents.preview.idCopied', 'Document ID copied to clipboard'));
    } catch {
      toast.error(t('common.copyFailed', 'Failed to copy'));
    }
  }, [documentId, t]);

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
    if (isPdfDocument && pdfContent?.markdown_content) {
      return { ...document, content: pdfContent.markdown_content };
    }
    return document;
  }, [document, isPdfDocument, pdfContent?.markdown_content]);

  // Derived status values (safe to compute even if document is null)
  const status = (document?.status || 'completed') as DocumentStatus;
  const statusInfo = statusConfig[status] || statusConfig.completed;
  const StatusIcon = statusInfo.icon;
  const isFailed = status === 'failed' || status === 'partial_failure';

  // Loading state
  if (isLoading) {
    return (
      <div className="flex flex-col h-screen">
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
      <div className="flex flex-col h-screen">
        <ErrorHeader />
        <div className="flex-1 flex items-center justify-center p-8">
          <ErrorContent error={error as Error} onRetry={refetch} />
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen overflow-hidden">
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

        {isFailed && document.error_message && (
          <div className="px-3 py-2 bg-destructive/10 border-t">
            <p className="text-xs text-destructive">{document.error_message}</p>
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
                  />
                }
                rightPanel={
                  // OODA-91: Show loading state while PDF markdown is being fetched
                  isPdfContentLoading ? (
                    <div className="flex items-center justify-center h-full">
                      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                    </div>
                  ) : (
                    <ContentRenderer 
                      document={documentWithContent} 
                      highlightText={highlightText}
                      startLine={startLine}
                      endLine={endLine}
                    />
                  )
                }
              />
            ) : (
              /* Non-PDF documents show ContentRenderer only */
              <ContentRenderer 
                document={documentWithContent} 
                highlightText={highlightText}
                startLine={startLine}
                endLine={endLine}
              />
            )}
          </div>

          {/* Metadata Sidebar - Resizable (hidden for PDF side-by-side to maximize content) */}
          {!isPdfDocument && (
            <ResizablePanel
              side="right"
              defaultWidth={520}
              minWidth={400}
              maxWidth={900}
              storageKey="document-detail-sidebar-width"
              ariaLabel="Resize metadata sidebar"
            >
              <MetadataSidebar document={document} />
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
                />
              </TabsContent>
            )}
            <TabsContent value="content" className="flex-1 overflow-auto m-0 mt-0">
              {/* OODA-91: Show loading state for PDF markdown on mobile */}
              {isPdfDocument && isPdfContentLoading ? (
                <div className="flex items-center justify-center h-full">
                  <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
                </div>
              ) : (
                <ContentRenderer 
                  document={documentWithContent} 
                  highlightText={highlightText}
                  startLine={startLine}
                  endLine={endLine}
                />
              )}
            </TabsContent>
            <TabsContent value="metadata" className="flex-1 overflow-hidden m-0 mt-0">
              <MetadataSidebar document={document} />
            </TabsContent>
          </Tabs>
        </div>
      </div>
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
