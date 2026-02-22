/**
 * @module PDFViewer
 * @description Reusable PDF viewer component using react-pdf.
 * Displays PDF documents with pagination, zoom, and scroll controls.
 *
 * @implements SPEC-002 - Document Viewer with PDF display
 * @implements FEAT0711 - PDF rendering with react-pdf
 * @implements FEAT0712 - Page navigation controls
 * @implements FEAT0713 - Zoom controls
 *
 * @enforces BR0711 - Smooth scrolling within container
 * @enforces BR0712 - Responsive width handling
 *
 * @see {@link docs/features.md} FEAT0711-0713
 */
'use client';

import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { cn } from '@/lib/utils';
import {
    ChevronLeft,
    ChevronRight,
    Loader2,
    Maximize2,
    Minimize2,
    XCircle,
    ZoomIn,
    ZoomOut,
} from 'lucide-react';
import dynamic from 'next/dynamic';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import 'react-pdf/dist/Page/AnnotationLayer.css';
import 'react-pdf/dist/Page/TextLayer.css';

// PDF file source type - matches react-pdf File type
// Can be: URL string, object with url, object with data (ArrayBuffer/TypedArray)
type PDFFileSource = string | { url: string } | { data: ArrayBuffer | Uint8Array } | null;

// Dynamic import to avoid SSR issues with react-pdf
const Document = dynamic(() => import('react-pdf').then(mod => mod.Document), {
  ssr: false,
  loading: () => <PDFLoadingSkeleton />,
});

const Page = dynamic(() => import('react-pdf').then(mod => mod.Page), {
  ssr: false,
});

// Configure PDF.js worker - must be done at module level
if (typeof window !== 'undefined') {
  import('react-pdf').then(({ pdfjs }) => {
    pdfjs.GlobalWorkerOptions.workerSrc = `//unpkg.com/pdfjs-dist@${pdfjs.version}/build/pdf.worker.min.mjs`;
  });
}

interface PDFViewerProps {
  /** PDF source - URL string, URL object, or data object with ArrayBuffer/Uint8Array */
  file: PDFFileSource;
  /** Optional class name for container */
  className?: string;
  /** Initial page number (1-indexed) */
  initialPage?: number;
  /** Initial zoom scale (1.0 = 100%) */
  initialScale?: number;
  /** Whether to show toolbar */
  showToolbar?: boolean;
  /** Fixed width for the PDF (auto if not provided) */
  width?: number;
  /** Fixed height for container (enables scrolling) */
  height?: number;
  /** Called when PDF loads successfully */
  onLoadSuccess?: (numPages: number) => void;
  /** Called when PDF load fails */
  onLoadError?: (error: Error) => void;
}

function PDFLoadingSkeleton() {
  return (
    <div className="flex flex-col items-center justify-center p-8 space-y-4">
      <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      <Skeleton className="h-[400px] w-[300px]" />
    </div>
  );
}

function PDFErrorState({ error, onRetry }: { error: string; onRetry?: () => void }) {
  const { t } = useTranslation();
  
  // WHY: Detect specific error types to show actionable messages instead of raw errors.
  // react-pdf throws "ResponseException: Unexpected server response (404)" when PDF is not found.
  const is404 = error.includes('404') || error.includes('not found');
  const isNetworkError = error.includes('NetworkError') || error.includes('Failed to fetch') || error.includes('network');
  
  const displayMessage = is404
    ? t('documents.viewer.pdfNotFound', 'PDF file is not available. The file may have been removed or processing may not be complete.')
    : isNetworkError
      ? t('documents.viewer.pdfNetworkError', 'Unable to connect to the server. Please check your connection and try again.')
      : error;

  return (
    <div className="flex flex-col items-center justify-center p-8 space-y-4 text-center">
      <div className="rounded-full bg-muted p-3">
        <XCircle className="h-6 w-6 text-muted-foreground" />
      </div>
      <div className="space-y-1">
        <p className="text-sm font-medium text-muted-foreground">
          {is404 
            ? t('documents.viewer.pdfUnavailable', 'PDF Unavailable')
            : t('documents.viewer.loadError', 'Failed to Load PDF')}
        </p>
        <p className="text-xs text-muted-foreground/70 max-w-sm">{displayMessage}</p>
      </div>
      {onRetry && !is404 && (
        <Button variant="outline" size="sm" onClick={onRetry}>
          {t('common.retry', 'Retry')}
        </Button>
      )}
    </div>
  );
}

/**
 * PDFViewer component for displaying PDF documents.
 *
 * Uses react-pdf (based on Mozilla pdf.js) for high-quality PDF rendering.
 * Supports pagination, zoom controls, and responsive width.
 */
export function PDFViewer({
  file,
  className,
  initialPage = 1,
  initialScale = 1.0,
  showToolbar = true,
  width,
  height,
  onLoadSuccess,
  onLoadError,
}: PDFViewerProps) {
  const { t } = useTranslation();
  const [numPages, setNumPages] = useState<number>(0);
  const [pageNumber, setPageNumber] = useState<number>(initialPage);
  const [scale, setScale] = useState<number>(initialScale);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [isFullWidth, setIsFullWidth] = useState<boolean>(false);
  // WHY: Pre-check the URL before mounting <Document> so that PDF.js never
  // attempts to fetch a 404 URL. Without this guard, react-pdf's internal worker
  // logs "ResponseException: Unexpected server response (404)" to the console even
  // though we handle the error in onLoadError. The HEAD request is cheap (no body
  // download) and prevents the noisy console warning entirely.
  const [urlOk, setUrlOk] = useState<boolean | null>(null);

  useEffect(() => {
    // Only pre-check simple URL strings; data/object sources skip the check.
    const url = typeof file === 'string' ? file : (file as { url?: string } | null)?.url;
    if (!url) {
      setUrlOk(true);
      return;
    }
    setUrlOk(null); // reset on file change
    let cancelled = false;
    fetch(url, { method: 'HEAD' })
      .then((res) => {
        if (!cancelled) setUrlOk(res.ok);
        if (!res.ok && !cancelled) {
          setError(`ResponseException: Unexpected server response (${res.status})`);
          setIsLoading(false);
        }
      })
      .catch(() => {
        // Network error — let react-pdf handle it via onLoadError
        if (!cancelled) setUrlOk(true);
      });
    return () => { cancelled = true; };
  }, [file]);

  const handleLoadSuccess = useCallback(({ numPages }: { numPages: number }) => {
    setNumPages(numPages);
    setIsLoading(false);
    setError(null);
    onLoadSuccess?.(numPages);
  }, [onLoadSuccess]);

  const handleLoadError = useCallback((err: Error) => {
    setError(err.message || 'Failed to load PDF');
    setIsLoading(false);
    onLoadError?.(err);
  }, [onLoadError]);

  const goToPreviousPage = useCallback(() => {
    setPageNumber(prev => Math.max(1, prev - 1));
  }, []);

  const goToNextPage = useCallback(() => {
    setPageNumber(prev => Math.min(numPages, prev + 1));
  }, [numPages]);

  const zoomIn = useCallback(() => {
    setScale(prev => Math.min(3.0, prev + 0.25));
  }, []);

  const zoomOut = useCallback(() => {
    setScale(prev => Math.max(0.5, prev - 0.25));
  }, []);

  const toggleFullWidth = useCallback(() => {
    setIsFullWidth(prev => !prev);
  }, []);

  if (!file) {
    return (
      <div className="flex items-center justify-center p-8 text-muted-foreground">
        {t('documents.viewer.noFile', 'No PDF file selected')}
      </div>
    );
  }

  if (error) {
    return <PDFErrorState error={error} onRetry={() => { setError(null); setUrlOk(null); }} />;
  }

  // WHY: Show loading skeleton while the HEAD pre-check is in flight or urlOk is false
  // to avoid mounting <Document> prematurely which would trigger PDF.js console warnings.
  if (urlOk === null) {
    return (
      <div className={cn('flex flex-col h-full min-h-0', className)}>
        <PDFLoadingSkeleton />
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col h-full min-h-0', className)}>
      {/* Toolbar */}
      {showToolbar && (
        <div className="flex items-center justify-between gap-2 p-2 border-b bg-muted/30">
          {/* Page Navigation */}
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={goToPreviousPage}
              disabled={pageNumber <= 1 || isLoading}
              title={t('documents.viewer.previousPage', 'Previous page')}
            >
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <span className="text-sm text-muted-foreground min-w-[80px] text-center">
              {isLoading ? '...' : `${pageNumber} / ${numPages}`}
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={goToNextPage}
              disabled={pageNumber >= numPages || isLoading}
              title={t('documents.viewer.nextPage', 'Next page')}
            >
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>

          {/* Zoom Controls */}
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={zoomOut}
              disabled={scale <= 0.5 || isLoading}
              title={t('documents.viewer.zoomOut', 'Zoom out')}
            >
              <ZoomOut className="h-4 w-4" />
            </Button>
            <span className="text-sm text-muted-foreground min-w-[50px] text-center">
              {Math.round(scale * 100)}%
            </span>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={zoomIn}
              disabled={scale >= 3.0 || isLoading}
              title={t('documents.viewer.zoomIn', 'Zoom in')}
            >
              <ZoomIn className="h-4 w-4" />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={toggleFullWidth}
              title={isFullWidth ? t('documents.viewer.fitWidth', 'Fit width') : t('documents.viewer.fullWidth', 'Full width')}
            >
              {isFullWidth ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
            </Button>
          </div>
        </div>
      )}

      {/* PDF Content - mousewheel scrollable
         WHY: flex-1 + min-h-0 lets the scroll area shrink to fit the parent.
         An explicit height is only set when the height prop is provided;
         otherwise flex handles it so the PDF page is fully scrollable. */}
      <div
        className={cn(
          'flex-1 min-h-0 overflow-y-auto overflow-x-hidden',
          'scroll-smooth bg-muted/10'
        )}
        style={{ 
          ...(height ? { height: `${height}px` } : {}),
          WebkitOverflowScrolling: 'touch'
        }}
      >
        <div className="flex justify-center py-4">
          {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
          <Document
            file={file as any}
            onLoadSuccess={handleLoadSuccess}
            onLoadError={handleLoadError}
            loading={<PDFLoadingSkeleton />}
            className="pdf-document"
          >
            <Page
              pageNumber={pageNumber}
              scale={scale}
              width={isFullWidth ? undefined : width}
              className="shadow-md"
              renderTextLayer={true}
              renderAnnotationLayer={true}
              loading={
                <div className="flex items-center justify-center p-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              }
            />
          </Document>
        </div>
      </div>
    </div>
  );
}

export default PDFViewer;
